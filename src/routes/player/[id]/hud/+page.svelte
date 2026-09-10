<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    mpvSetPause,
    mpvSeek,
    mpvSetVolume,
    getTracks,
    mpvSetSubtitleTrack,
    mpvSetAudioTrack,
    goBackToItem,
    toggleMainFullscreen,
    enterPip,
    restoreFromPip,
  } from "$lib/jellyfinClient";
  import type { Track } from "$lib/types";

  // This page renders in its own always-on-top, transparent Tauri window
  // (see create_hud_window's doc comment in the Rust backend) that sits
  // above mpv's video surface at all times. That's what makes it possible
  // for the video to always fill the entire main window edge to edge,
  // completely independent of whatever this HUD is doing -- there is no
  // shared layout between this window and the video at all, so nothing here
  // can ever affect the video's size. Which also means the audio/captions
  // pickers below can be true floating dropdowns that pop up over the
  // player, instead of needing to squeeze into the control bar's own
  // footprint the way an earlier version of this had to.

  let paused = $state(false);
  let timePos = $state(0);
  let duration = $state(0);
  let volume = $state(100);

  let audioTracks = $state<Track[]>([]);
  let subtitleTracks = $state<Track[]>([]);
  let selectedAudioId = $state<number | null>(null);
  let selectedSubtitleId = $state<number | null>(null);
  // Which track picker (if any) is open. Floating dropdown, anchored to its
  // trigger button -- see the "menu" styles below.
  let activePanel = $state<"none" | "audio" | "captions">("none");
  let isFullscreen = $state(false);

  // Whether the top bar / control bar are shown. Auto-hides after
  // inactivity like a normal video player HUD.
  let hudVisible = $state(true);
  let hudTimer: ReturnType<typeof setTimeout> | undefined;
  const HUD_HIDE_DELAY_MS = 4000;

  let unlistenProperty: UnlistenFn | undefined;
  let seeking = false;

  // Whether this window is currently the small floating PiP overlay rather
  // than the full-window HUD -- toggled by the Rust side (see enter_pip /
  // exit_pip_if_active) once it's actually finished resizing this window, so
  // the layout switch and the window's actual size change stay in lockstep.
  let pipMode = $state(false);
  let unlistenPip: UnlistenFn | undefined;

  function resetHudTimer() {
    hudVisible = true;
    if (hudTimer) clearTimeout(hudTimer);
    if (activePanel !== "none") return;
    hudTimer = setTimeout(() => {
      if (activePanel === "none") hudVisible = false;
    }, HUD_HIDE_DELAY_MS);
  }

  async function loadTracks() {
    try {
      const tracks = await getTracks();
      audioTracks = tracks.filter((t) => t.type === "audio");
      subtitleTracks = tracks.filter((t) => t.type === "sub");
      selectedAudioId = audioTracks.find((t) => t.selected)?.id ?? null;
      selectedSubtitleId = subtitleTracks.find((t) => t.selected)?.id ?? null;
    } catch {
      // Track list isn't critical to playback; leave the selectors empty if it fails.
    }
  }

  onMount(async () => {
    let tracksLoaded = false;

    unlistenProperty = await listen<{ name: string; value: unknown }>("mpv://property-change", (event) => {
      const { name, value } = event.payload;
      if (name === "time-pos" && !seeking && typeof value === "number") timePos = value;
      if (name === "pause" && typeof value === "boolean") paused = value;
      if (name === "duration" && typeof value === "number") {
        duration = value;
        if (value > 0 && !tracksLoaded) {
          tracksLoaded = true;
          loadTracks();
        }
      }
    });

    unlistenPip = await listen<boolean>("player://pip-changed", (event) => {
      pipMode = event.payload;
    });

    window.addEventListener("keydown", handleKeydown);
    resetHudTimer();
  });

  onDestroy(() => {
    unlistenProperty?.();
    unlistenPip?.();
    window.removeEventListener("keydown", handleKeydown);
    if (hudTimer) clearTimeout(hudTimer);
  });

  // A guaranteed-reliable way out of fullscreen that doesn't depend on the
  // mouse at all.
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      if (activePanel !== "none") {
        closePanel();
      } else if (isFullscreen) {
        toggleFullscreen();
      }
      return;
    }
    resetHudTimer();
  }

  // The Rust side resizes this window and emits "player://pip-changed" once
  // it's actually done, which is what flips `pipMode` -- not this call
  // directly, so the layout switch never gets ahead of the window's real size.
  function pip() {
    enterPip().catch(() => {});
  }

  function restorePip() {
    restoreFromPip().catch(() => {});
  }

  function seekRelative(deltaSeconds: number) {
    mpvSeek(Math.max(0, timePos + deltaSeconds)).catch(() => {});
  }

  // In PiP mode this window's own chrome is the only drag/resize handle the
  // floating video has -- see sync_pip_from_hud's doc comment in the Rust
  // backend for why the video itself no longer handles either now that this
  // HUD covers it completely. `startDragging` hands off to the OS's own move
  // loop, which is what then fires the "Moved" events the backend listens
  // for to keep the video glued to this window.
  function startPipDrag(event: MouseEvent) {
    if (event.button !== 0) return;
    getCurrentWindow()
      .startDragging()
      .catch(() => {});
  }

  // The invisible corner handles around the PiP overlay -- same idea as
  // startPipDrag, but handing off to the OS's native resize loop instead,
  // which fires "Resized" (and, since a corner drag moves the window's
  // origin too) "Moved" events for the backend to follow and, in the
  // Resized case, correct back to a fixed aspect ratio. Corner-only (no
  // plain edge handles) since aspect-locked resizing only has one sensible
  // interpretation for a diagonal drag -- a single edge alone doesn't say
  // which of width/height should lead.
  function startPipResize(direction: "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest") {
    return (event: MouseEvent) => {
      if (event.button !== 0) return;
      event.stopPropagation();
      getCurrentWindow()
        .startResizeDragging(direction)
        .catch(() => {});
    };
  }

  // Stops a button press over the PiP overlay from also being interpreted as
  // the start of a window drag (see startPipDrag) -- mousedown bubbles up to
  // the overlay's own drag handler otherwise, which would swallow the click.
  function stopDragPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  async function toggleFullscreen() {
    try {
      isFullscreen = await toggleMainFullscreen();
    } catch {
      // leave isFullscreen as-is
    }
    resetHudTimer();
  }

  function togglePause() {
    paused = !paused;
    mpvSetPause(paused).catch(() => {});
    resetHudTimer();
  }

  function handleSeekInput(event: Event) {
    seeking = true;
    timePos = Number((event.target as HTMLInputElement).value);
    resetHudTimer();
  }

  function handleSeekCommit(event: Event) {
    const seconds = Number((event.target as HTMLInputElement).value);
    mpvSeek(seconds).catch(() => {});
    seeking = false;
  }

  function handleVolumeInput(event: Event) {
    volume = Number((event.target as HTMLInputElement).value);
    mpvSetVolume(volume).catch(() => {});
    resetHudTimer();
  }

  // mpv reports language as a raw ISO 639 code (2- or 3-letter, depending on
  // how the file was muxed) -- shown on its own that reads as an abbreviated
  // fragment rather than a real title, so it's spelled out in full here.
  const LANGUAGE_NAMES: Record<string, string> = {
    en: "English", eng: "English",
    ja: "Japanese", jpn: "Japanese",
    es: "Spanish", spa: "Spanish",
    fr: "French", fre: "French", fra: "French",
    de: "German", ger: "German", deu: "German",
    it: "Italian", ita: "Italian",
    pt: "Portuguese", por: "Portuguese",
    ko: "Korean", kor: "Korean",
    zh: "Chinese", chi: "Chinese", zho: "Chinese",
    ru: "Russian", rus: "Russian",
    nl: "Dutch", dut: "Dutch", nld: "Dutch",
    ar: "Arabic", ara: "Arabic",
    hi: "Hindi", hin: "Hindi",
    sv: "Swedish", swe: "Swedish",
    no: "Norwegian", nor: "Norwegian",
    da: "Danish", dan: "Danish",
    fi: "Finnish", fin: "Finnish",
    pl: "Polish", pol: "Polish",
    tr: "Turkish", tur: "Turkish",
    th: "Thai", tha: "Thai",
    vi: "Vietnamese", vie: "Vietnamese",
    id: "Indonesian", ind: "Indonesian",
    he: "Hebrew", heb: "Hebrew",
    cs: "Czech", cze: "Czech", ces: "Czech",
    hu: "Hungarian", hun: "Hungarian",
    el: "Greek", gre: "Greek", ell: "Greek",
    ro: "Romanian", rum: "Romanian", ron: "Romanian",
    uk: "Ukrainian", ukr: "Ukrainian",
    und: "Unknown",
  };

  function languageName(code: string): string {
    return LANGUAGE_NAMES[code.toLowerCase()] ?? code.toUpperCase();
  }

  function trackLabel(track: Track): string {
    const lang = track.lang ? languageName(track.lang) : null;
    if (track.title && lang) return `${track.title} (${lang})`;
    if (track.title) return track.title;
    if (lang) return lang;
    return `Track ${track.id}`;
  }

  function openPanel(panel: "audio" | "captions") {
    activePanel = activePanel === panel ? "none" : panel;
    resetHudTimer();
  }

  function closePanel() {
    activePanel = "none";
    resetHudTimer();
  }

  function chooseAudio(id: number) {
    selectedAudioId = id;
    mpvSetAudioTrack(id).catch(() => {});
    activePanel = "none";
    resetHudTimer();
  }

  function chooseSubtitle(id: number | null) {
    selectedSubtitleId = id;
    mpvSetSubtitleTrack(id).catch(() => {});
    activePanel = "none";
    resetHudTimer();
  }

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds)) return "0:00";
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function handleBack() {
    await goBackToItem(page.params.id!).catch(() => {});
  }
</script>

{#if pipMode}
  <div class="pip-overlay" role="presentation" onmousedown={startPipDrag}>
    <div class="pip-resize pip-resize-nw" role="presentation" onmousedown={startPipResize("NorthWest")}></div>
    <div class="pip-resize pip-resize-ne" role="presentation" onmousedown={startPipResize("NorthEast")}></div>
    <div class="pip-resize pip-resize-sw" role="presentation" onmousedown={startPipResize("SouthWest")}></div>
    <div class="pip-resize pip-resize-se" role="presentation" onmousedown={startPipResize("SouthEast")}></div>

    <button
      class="pip-restore"
      onmousedown={stopDragPropagation}
      onclick={restorePip}
      title="Return to FastFin"
    >
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6"></path><path d="M9 21H3v-6"></path><path d="M21 3l-7 7"></path><path d="M3 21l7-7"></path></svg>
    </button>

    <div class="pip-controls">
      <button
        class="pip-btn"
        onmousedown={stopDragPropagation}
        onclick={() => seekRelative(-7)}
        title="Back 7 seconds"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 19 2 12 11 5 11 19"></polygon><polygon points="22 19 13 12 22 5 22 19"></polygon></svg>
      </button>

      <button
        class="pip-btn pip-btn-main"
        onmousedown={stopDragPropagation}
        onclick={togglePause}
        title={paused ? "Play" : "Pause"}
      >
        {#if paused}
          <svg width="26" height="26" viewBox="0 0 24 24" fill="white"><path d="M6 4l15 8-15 8z"></path></svg>
        {:else}
          <svg width="26" height="26" viewBox="0 0 24 24" fill="white"><rect x="6" y="4" width="4" height="16" rx="1"></rect><rect x="14" y="4" width="4" height="16" rx="1"></rect></svg>
        {/if}
      </button>

      <button
        class="pip-btn"
        onmousedown={stopDragPropagation}
        onclick={() => seekRelative(5)}
        title="Forward 5 seconds"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 5 22 12 13 19 13 5"></polygon><polygon points="2 5 11 12 2 19 2 5"></polygon></svg>
      </button>
    </div>
  </div>
{:else}
<div class="hud-root" onmousemove={resetHudTimer} role="presentation">
  <div class="top-bar chrome" class:hud-hidden={!hudVisible}>
    <button class="icon" onclick={handleBack} title="Back">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
    </button>
  </div>

  <div class="spacer"></div>

  <div class="controls chrome" class:hud-hidden={!hudVisible} role="toolbar" aria-label="Playback controls" tabindex="-1">
    <button class="icon" onclick={togglePause} title={paused ? "Play" : "Pause"}>
      {#if paused}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M6 4l15 8-15 8z"></path></svg>
      {:else}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="4" width="4" height="16" rx="1"></rect><rect x="14" y="4" width="4" height="16" rx="1"></rect></svg>
      {/if}
    </button>
    <span class="time">{formatTime(timePos)}</span>
    <input
      class="seek"
      type="range"
      min="0"
      max={duration || 0}
      step="1"
      value={timePos}
      oninput={handleSeekInput}
      onchange={handleSeekCommit}
    />
    <span class="time">{formatTime(duration)}</span>
    <input
      class="volume"
      type="range"
      min="0"
      max="100"
      step="1"
      value={volume}
      oninput={handleVolumeInput}
    />

    {#if audioTracks.length > 1}
      <div class="menu-anchor">
        <button
          class="icon"
          onclick={() => openPanel("audio")}
          title="Audio track"
        >
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><line x1="6" y1="16" x2="6" y2="10"></line><line x1="12" y1="19" x2="12" y2="6"></line><line x1="18" y1="15" x2="18" y2="12"></line></svg>
        </button>
        {#if activePanel === "audio"}
          <div class="menu">
            <div class="menu-label">Audio</div>
            {#each audioTracks as track (track.id)}
              <button class="menu-item" class:selected={selectedAudioId === track.id} onclick={() => chooseAudio(track.id)}>
                {trackLabel(track)}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    {#if subtitleTracks.length > 0}
      <div class="menu-anchor">
        <button
          class="icon cc"
          onclick={() => openPanel("captions")}
          title="Captions"
        >CC</button>
        {#if activePanel === "captions"}
          <div class="menu">
            <div class="menu-label">Captions</div>
            <button class="menu-item" class:selected={selectedSubtitleId === null} onclick={() => chooseSubtitle(null)}>
              Off
            </button>
            {#each subtitleTracks as track (track.id)}
              <button class="menu-item" class:selected={selectedSubtitleId === track.id} onclick={() => chooseSubtitle(track.id)}>
                {trackLabel(track)}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <button class="icon" onclick={pip} title="Picture in picture">
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="5" width="18" height="14" rx="1.5"></rect><rect x="12" y="12" width="7" height="5" rx="1" fill="currentColor" stroke="none"></rect></svg>
    </button>
    <button class="icon" onclick={toggleFullscreen} title={isFullscreen ? "Exit fullscreen" : "Fullscreen"}>
      {#if isFullscreen}
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3v4a1 1 0 0 1-1 1H4M15 3v4a1 1 0 0 0 1 1h4M9 21v-4a1 1 0 0 0-1-1H4M15 21v-4a1 1 0 0 1 1-1h4"></path></svg>
      {:else}
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H4a1 1 0 0 0-1 1v4M16 3h4a1 1 0 0 1 1 1v4M8 21H4a1 1 0 0 1-1-1v-4M16 21h4a1 1 0 0 0 1-1v-4"></path></svg>
      {/if}
    </button>
  </div>
</div>
{/if}

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
  }

  .hud-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: transparent;
  }

  .spacer {
    flex: 1;
    min-height: 0;
  }

  .chrome {
    transition: opacity 0.25s ease;
  }

  .chrome.hud-hidden {
    opacity: 0;
  }

  .top-bar {
    display: flex;
    align-items: center;
    padding: 0.5rem 0.75rem;
    flex-shrink: 0;
    /* Semi-transparent so the video underneath is faintly visible through
       the HUD's own chrome, per the requested look. */
    background: linear-gradient(180deg, rgba(10, 10, 14, 0.55), rgba(10, 10, 14, 0));
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    background: rgba(24, 24, 28, 0.55);
    border-top: 1px solid rgba(44, 44, 48, 0.6);
    height: 56px;
    box-sizing: border-box;
    flex-shrink: 0;
    backdrop-filter: blur(6px);
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text);
    padding: 0.4rem;
    border-radius: 999px;
    flex-shrink: 0;
  }

  .icon:hover {
    background: var(--bg-hover);
  }

  .icon.cc {
    width: 30px;
    height: 22px;
    border: 1.7px solid currentColor;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.02em;
  }

  .time {
    font-size: 0.8rem;
    color: var(--text-dim);
    min-width: 3.5em;
    text-align: center;
    flex-shrink: 0;
  }

  .seek {
    flex: 1;
    accent-color: var(--accent);
  }

  .volume {
    width: 100px;
    accent-color: var(--accent);
    flex-shrink: 0;
  }

  .menu-anchor {
    position: relative;
    flex-shrink: 0;
  }

  /* Floats above the control bar and onto the player -- safe to do now that
     this HUD is a separate overlay window from the video (see the doc
     comment at the top of the script), so nothing here can ever push on the
     video's size the way it would have in the single-window design this
     replaced. */
  .menu {
    position: absolute;
    bottom: calc(100% + 12px);
    right: 0;
    min-width: 220px;
    max-width: 360px;
    max-height: 60vh;
    overflow-y: auto;
    background: rgba(24, 24, 28, 0.92);
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    padding: 8px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.55);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .menu-label {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 4px 8px;
  }

  .menu-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 13px;
    padding: 8px;
    border-radius: 6px;
    white-space: normal;
    word-break: break-word;
  }

  .menu-item:hover {
    background: var(--bg-hover);
  }

  .menu-item.selected {
    color: var(--accent);
    font-weight: 600;
  }

  /* The compact overlay shown instead of the full HUD while this window is
     the small floating PiP window -- transparent (so the video shows through
     untouched) until hovered, at which point it dims and reveals white
     controls. Pure CSS :hover rather than the full HUD's JS-timer-driven
     auto-hide: there's no "activity" to track here beyond the mouse simply
     being over the window at all. */
  .pip-overlay {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    transition: background 0.15s ease;
  }

  .pip-overlay:hover {
    background: rgba(0, 0, 0, 0.55);
  }

  .pip-controls {
    display: flex;
    align-items: center;
    gap: 20px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .pip-overlay:hover .pip-controls {
    opacity: 1;
  }

  .pip-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: white;
    padding: 6px;
    border-radius: 999px;
  }

  .pip-btn:hover {
    background: rgba(255, 255, 255, 0.18);
  }

  .pip-btn-main {
    padding: 10px;
  }

  .pip-restore {
    position: absolute;
    top: 16px;
    right: 16px;
    background: rgba(0, 0, 0, 0.35);
    border: none;
    color: white;
    padding: 5px;
    border-radius: 6px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .pip-overlay:hover .pip-restore {
    opacity: 1;
  }

  .pip-restore:hover {
    background: rgba(255, 255, 255, 0.25);
  }

  /* Invisible drag-to-resize corners around the PiP window -- there's no OS
     window chrome to grab (it's borderless), so these stand in for it.
     Corner-only, not full edges: resizing is locked to a fixed aspect ratio
     (see enforce_pip_aspect in the Rust backend), and a diagonal drag is the
     only gesture that unambiguously implies both dimensions changing
     together. */
  .pip-resize {
    position: absolute;
    z-index: 2;
    width: 14px;
    height: 14px;
  }

  .pip-resize-nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  .pip-resize-ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  .pip-resize-sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  .pip-resize-se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
</style>

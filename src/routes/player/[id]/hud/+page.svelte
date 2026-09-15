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
    getItem,
    getEpisodes,
    getNextEpisode,
    getSkipSegments,
    playItem,
  } from "$lib/jellyfinClient";
  import { episodeCode, type Item, type SkipSegment, type Track } from "$lib/types";
  import {
    getVolume as loadStoredVolume,
    setVolume as storeVolume,
    getPreferredAudio,
    getPreferredSubtitle,
    languageMatches,
  } from "$lib/prefs";

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
  // Volume to restore when unmuting (M key). 0 = not muted.
  let volumeBeforeMute = 0;

  let audioTracks = $state<Track[]>([]);
  let subtitleTracks = $state<Track[]>([]);
  let selectedAudioId = $state<number | null>(null);
  let selectedSubtitleId = $state<number | null>(null);
  // Which panel (if any) is open. The track pickers are floating dropdowns
  // anchored to their trigger button; the episode picker is a taller list
  // in the same style -- see the "menu" styles below.
  let activePanel = $state<"none" | "audio" | "captions" | "episodes">("none");
  let isFullscreen = $state(false);

  // Whether the top bar / control bar are shown. Auto-hides after
  // inactivity like a normal video player HUD.
  let hudVisible = $state(true);
  let hudTimer: ReturnType<typeof setTimeout> | undefined;
  const HUD_HIDE_DELAY_MS = 4000;
  const SEEK_STEP_S = 10;
  const VOLUME_STEP = 5;

  let unlistenProperty: UnlistenFn | undefined;
  let unlistenKeydown: UnlistenFn | undefined;
  let seeking = false;

  // Whether this window is currently the small floating PiP overlay rather
  // than the full-window HUD -- toggled by the Rust side (see enter_pip /
  // exit_pip_if_active) once it's actually finished resizing this window, so
  // the layout switch and the window's actual size change stay in lockstep.
  let pipMode = $state(false);
  let unlistenPip: UnlistenFn | undefined;

  // What's playing. Only episodes get the next-episode / episode-picker /
  // skip-segment machinery below; for a movie these all stay empty.
  let currentItem = $state<Item | null>(null);
  let nextEpisode = $state<Item | null>(null);
  let seasonEpisodes = $state<Item[] | null>(null);
  let episodesError = $state("");

  // Intro Skipper (or native media segment) ranges for this item, in
  // seconds. Empty when the plugin isn't installed or hasn't analyzed the
  // episode yet -- then no skip button ever shows, which is the right
  // fallback.
  let segments = $state<SkipSegment[]>([]);
  let activeSegment = $derived(
    segments.find((s) => timePos >= s.start && timePos < s.end - 0.5) ?? null,
  );
  let skippableSegment = $derived(
    activeSegment && activeSegment.kind !== "Credits" ? activeSegment : null,
  );
  let inCredits = $derived(activeSegment?.kind === "Credits");
  let eofReached = $state(false);

  // Autoplay-next card: appears when the credits segment starts (or, without
  // credits data, when the file actually ends), counts down, then plays the
  // next episode. Cancelling keeps it hidden for the rest of this episode.
  const UP_NEXT_SECONDS = 10;
  let upNextVisible = $state(false);
  let upNextDismissed = $state(false);
  let countdown = $state(UP_NEXT_SECONDS);
  let countdownTimer: ReturnType<typeof setInterval> | undefined;
  let switching = false;

  function resetHudTimer() {
    hudVisible = true;
    if (hudTimer) clearTimeout(hudTimer);
    if (activePanel !== "none") return;
    hudTimer = setTimeout(() => {
      if (activePanel === "none") hudVisible = false;
    }, HUD_HIDE_DELAY_MS);
  }

  // Auto-selects audio/subtitle tracks matching the user's language
  // preferences (settings popover in the main window) once the track list
  // is known. Leaves mpv's own choice alone whenever no preference is set
  // or nothing matches, so this can only ever improve on the default.
  function applyLanguagePreferences() {
    const prefAudio = getPreferredAudio();
    if (prefAudio) {
      const match = audioTracks.find((t) => languageMatches(t.lang, prefAudio));
      if (match && match.id !== selectedAudioId) {
        selectedAudioId = match.id;
        mpvSetAudioTrack(match.id).catch(() => {});
      }
    }
    const prefSub = getPreferredSubtitle();
    if (prefSub === "off") {
      if (selectedSubtitleId !== null) {
        selectedSubtitleId = null;
        mpvSetSubtitleTrack(null).catch(() => {});
      }
    } else if (prefSub) {
      const match = subtitleTracks.find((t) => languageMatches(t.lang, prefSub));
      if (match && match.id !== selectedSubtitleId) {
        selectedSubtitleId = match.id;
        mpvSetSubtitleTrack(match.id).catch(() => {});
      }
    }
  }

  async function loadTracks() {
    try {
      const tracks = await getTracks();
      audioTracks = tracks.filter((t) => t.type === "audio");
      subtitleTracks = tracks.filter((t) => t.type === "sub");
      selectedAudioId = audioTracks.find((t) => t.selected)?.id ?? null;
      selectedSubtitleId = subtitleTracks.find((t) => t.selected)?.id ?? null;
      applyLanguagePreferences();
    } catch {
      // Track list isn't critical to playback; leave the selectors empty if it fails.
    }
  }

  async function loadItemContext(itemId: string) {
    // Segments are useful for movies too (some providers tag recaps or
    // credits on films), so they're fetched regardless of type.
    getSkipSegments(itemId).then((s) => (segments = s));
    try {
      currentItem = await getItem(itemId);
    } catch {
      return;
    }
    if (currentItem.Type === "Episode") {
      getNextEpisode(itemId)
        .then((next) => (nextEpisode = next))
        .catch(() => (nextEpisode = null));
    }
  }

  onMount(async () => {
    let tracksLoaded = false;

    unlistenProperty = await listen<{ name: string; value: unknown }>("mpv://property-change", (event) => {
      const { name, value } = event.payload;
      if (name === "time-pos" && !seeking && typeof value === "number") timePos = value;
      if (name === "pause" && typeof value === "boolean") paused = value;
      if (name === "eof-reached" && typeof value === "boolean") eofReached = value;
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

    // Keys arrive from the main window -- this overlay is non-focusable and
    // never receives keyboard input directly (see the main player page).
    unlistenKeydown = await listen<{ key: string; shift: boolean }>("player://keydown", (event) => {
      handleKey(event.payload.key, event.payload.shift);
    });

    // Restore last session's volume before the file starts making noise.
    volume = loadStoredVolume();
    mpvSetVolume(volume).catch(() => {});

    window.addEventListener("keydown", handleKeydown);
    resetHudTimer();
    loadItemContext(page.params.id!);
  });

  onDestroy(() => {
    unlistenProperty?.();
    unlistenPip?.();
    unlistenKeydown?.();
    window.removeEventListener("keydown", handleKeydown);
    if (hudTimer) clearTimeout(hudTimer);
    stopCountdown();
  });

  // Shows the autoplay card when the credits start or the file ends, as
  // long as there's something to play next and the user hasn't waved it
  // off for this episode.
  $effect(() => {
    if (!nextEpisode || upNextDismissed || pipMode) return;
    const due = inCredits || eofReached;
    if (due && !upNextVisible) showUpNext();
    // Seeking back out of the credits (to rewatch a scene, say) withdraws
    // the card and its countdown; it comes back when the credits do.
    if (!due && upNextVisible) {
      stopCountdown();
      upNextVisible = false;
    }
  });

  function showUpNext() {
    upNextVisible = true;
    countdown = UP_NEXT_SECONDS;
    hudVisible = true;
    stopCountdown();
    countdownTimer = setInterval(() => {
      countdown -= 1;
      if (countdown <= 0) {
        stopCountdown();
        playNext();
      }
    }, 1000);
  }

  function stopCountdown() {
    if (countdownTimer) clearInterval(countdownTimer);
    countdownTimer = undefined;
  }

  function dismissUpNext() {
    stopCountdown();
    upNextVisible = false;
    upNextDismissed = true;
    resetHudTimer();
  }

  function playNext() {
    if (!nextEpisode || switching) return;
    switching = true;
    stopCountdown();
    playItem(nextEpisode.Id).catch(() => (switching = false));
  }

  function skipSegment() {
    if (!skippableSegment) return;
    mpvSeek(skippableSegment.end).catch(() => {});
    resetHudTimer();
  }

  function segmentLabel(segment: SkipSegment): string {
    switch (segment.kind) {
      case "Intro":
        return "Skip Intro";
      case "Recap":
        return "Skip Recap";
      case "Preview":
        return "Skip Preview";
      case "Commercial":
        return "Skip Ad";
      default:
        return "Skip";
    }
  }

  // Direct keydown on this window is normally unreachable (non-focusable),
  // kept as a fallback for the main window's forwarded keys.
  function handleKeydown(event: KeyboardEvent) {
    handleKey(event.key, event.shiftKey);
  }

  function handleKey(key: string, shift: boolean) {
    switch (key) {
      case "Escape":
        if (upNextVisible) dismissUpNext();
        else if (activePanel !== "none") closePanel();
        else if (isFullscreen) toggleFullscreen();
        return;
      case " ":
      case "k":
      case "K":
        togglePause();
        break;
      case "ArrowLeft":
      case "j":
      case "J":
        seekRelative(shift ? -60 : -SEEK_STEP_S);
        break;
      case "ArrowRight":
      case "l":
      case "L":
        seekRelative(shift ? 60 : SEEK_STEP_S);
        break;
      case "ArrowUp":
        setVolume(volume + VOLUME_STEP);
        break;
      case "ArrowDown":
        setVolume(volume - VOLUME_STEP);
        break;
      case "m":
      case "M":
        toggleMute();
        break;
      case "f":
      case "F":
        toggleFullscreen();
        return;
      case "s":
      case "S":
        if (skippableSegment) skipSegment();
        break;
      case "n":
      case "N":
        if (nextEpisode) playNext();
        break;
      case "e":
      case "E":
        if (currentItem?.Type === "Episode") openPanel("episodes");
        break;
      default:
        break;
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
    const target = Math.max(0, Math.min(duration || Infinity, timePos + deltaSeconds));
    timePos = target;
    mpvSeek(target).catch(() => {});
  }

  function setVolume(next: number) {
    volume = Math.max(0, Math.min(100, Math.round(next)));
    if (volume > 0) volumeBeforeMute = 0;
    mpvSetVolume(volume).catch(() => {});
    storeVolume(volume);
  }

  function toggleMute() {
    if (volume > 0) {
      volumeBeforeMute = volume;
      volume = 0;
      mpvSetVolume(0).catch(() => {});
    } else {
      setVolume(volumeBeforeMute || 50);
    }
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
    setVolume(Number((event.target as HTMLInputElement).value));
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

  function openPanel(panel: "audio" | "captions" | "episodes") {
    activePanel = activePanel === panel ? "none" : panel;
    if (activePanel === "episodes" && seasonEpisodes === null) loadSeasonEpisodes();
    resetHudTimer();
  }

  function closePanel() {
    activePanel = "none";
    resetHudTimer();
  }

  // Fetched lazily the first time the picker opens -- a season's episode
  // list is only ever needed if the user actually reaches for it.
  async function loadSeasonEpisodes() {
    if (!currentItem?.SeriesId || !currentItem.SeasonId) return;
    try {
      seasonEpisodes = await getEpisodes(currentItem.SeriesId, currentItem.SeasonId);
    } catch (e) {
      episodesError = typeof e === "string" ? e : "Couldn't load episodes";
      seasonEpisodes = [];
    }
  }

  function chooseEpisode(id: string) {
    if (id === currentItem?.Id || switching) return;
    switching = true;
    activePanel = "none";
    playItem(id).catch(() => (switching = false));
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
    {#if currentItem}
      <span class="now-playing">
        {#if currentItem.Type === "Episode" && currentItem.SeriesName}
          <span class="np-series">{currentItem.SeriesName}</span>
          <span class="np-episode">{episodeCode(currentItem)} &middot; {currentItem.Name}</span>
        {:else}
          <span class="np-series">{currentItem.Name}</span>
        {/if}
      </span>
    {/if}
  </div>

  <div class="spacer"></div>

  <!-- Skip intro/recap and the autoplay-next card sit above the control bar
       and stay visible even when the rest of the chrome has auto-hidden --
       they're the whole point of looking at the screen at that moment. -->
  <div class="overlays">
    {#if skippableSegment && !upNextVisible}
      <button class="skip" onclick={skipSegment}>
        {segmentLabel(skippableSegment)}
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M5 4l10 8-10 8z"></path><rect x="17" y="4" width="3" height="16" rx="1"></rect></svg>
      </button>
    {/if}

    {#if upNextVisible && nextEpisode}
      <div class="up-next">
        <div class="up-next-label">Up next in {countdown}s</div>
        <div class="up-next-title">
          {episodeCode(nextEpisode)}{#if episodeCode(nextEpisode)}&nbsp;&middot;&nbsp;{/if}{nextEpisode.Name}
        </div>
        <div class="up-next-actions">
          <button class="up-next-play" onclick={playNext}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 4l15 8-15 8z"></path></svg>
            Play now
          </button>
          <button class="up-next-cancel" onclick={dismissUpNext}>Cancel</button>
        </div>
        <div class="up-next-track"><div class="up-next-fill" style={`width:${(1 - countdown / UP_NEXT_SECONDS) * 100}%`}></div></div>
      </div>
    {/if}
  </div>

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

    {#if currentItem?.Type === "Episode"}
      <div class="menu-anchor">
        <button class="icon" onclick={() => openPanel("episodes")} title="Episodes (E)">
          <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"></line><line x1="8" y1="12" x2="21" y2="12"></line><line x1="8" y1="18" x2="21" y2="18"></line><line x1="3" y1="6" x2="3.01" y2="6"></line><line x1="3" y1="12" x2="3.01" y2="12"></line><line x1="3" y1="18" x2="3.01" y2="18"></line></svg>
        </button>
        {#if activePanel === "episodes"}
          <div class="menu episodes">
            <div class="menu-label">
              {currentItem.SeriesName ?? "Episodes"}{#if currentItem.ParentIndexNumber != null}&nbsp;&middot;&nbsp;Season {currentItem.ParentIndexNumber}{/if}
            </div>
            {#if seasonEpisodes === null}
              <div class="menu-dim">Loading&hellip;</div>
            {:else if episodesError}
              <div class="menu-dim">{episodesError}</div>
            {:else}
              {#each seasonEpisodes as ep (ep.Id)}
                <button class="menu-item episode" class:selected={ep.Id === currentItem.Id} onclick={() => chooseEpisode(ep.Id)}>
                  <span class="ep-num">{ep.IndexNumber ?? ""}</span>
                  <span class="ep-name">{ep.Name}</span>
                  {#if ep.UserData?.Played}
                    <svg class="ep-watched" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                  {/if}
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
      {#if nextEpisode}
        <button class="icon" onclick={playNext} title={`Next: ${episodeCode(nextEpisode)} ${nextEpisode.Name} (N)`}>
          <svg width="17" height="17" viewBox="0 0 24 24" fill="currentColor"><path d="M5 4l10 8-10 8z"></path><rect x="17" y="4" width="3" height="16" rx="1"></rect></svg>
        </button>
      {/if}
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
    position: relative;
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

  .menu-dim {
    padding: 8px;
    font-size: 13px;
    color: var(--text-dim);
  }

  .menu.episodes {
    min-width: 320px;
    max-width: 420px;
  }

  .menu-item.episode {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .ep-num {
    flex-shrink: 0;
    min-width: 1.8em;
    text-align: right;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }

  .menu-item.selected .ep-num {
    color: var(--accent);
  }

  .ep-name {
    flex: 1;
  }

  .ep-watched {
    flex-shrink: 0;
    align-self: center;
    color: var(--text-dim);
  }

  .now-playing {
    display: flex;
    flex-direction: column;
    margin-left: 6px;
    line-height: 1.25;
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
  }

  .np-series {
    font-size: 14px;
    font-weight: 700;
  }

  .np-episode {
    font-size: 12px;
    color: var(--text-dim);
  }

  /* Skip button + up-next card: bottom-right, just above the control bar. */
  .overlays {
    position: absolute;
    right: 24px;
    bottom: 76px;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 12px;
    z-index: 3;
  }

  .skip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    font-size: 14px;
    font-weight: 700;
    color: #141018;
    background: rgba(255, 255, 255, 0.92);
    border: none;
    border-radius: 6px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.45);
  }

  .skip:hover {
    background: #fff;
  }

  .up-next {
    position: relative;
    overflow: hidden;
    width: 320px;
    padding: 14px 16px 18px;
    background: rgba(24, 24, 28, 0.92);
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.55);
  }

  .up-next-label {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 4px;
  }

  .up-next-title {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .up-next-actions {
    display: flex;
    gap: 8px;
  }

  .up-next-play {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 700;
    color: #141018;
    background: var(--text);
    border: none;
    border-radius: 6px;
  }

  .up-next-cancel {
    padding: 8px 14px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 6px;
  }

  .up-next-track {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: rgba(255, 255, 255, 0.12);
  }

  .up-next-fill {
    height: 100%;
    background: var(--accent);
    transition: width 1s linear;
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

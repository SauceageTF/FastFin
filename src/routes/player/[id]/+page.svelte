<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    startPlayback,
    resizeVideoSurface,
    stopPlayback,
    mpvSetPause,
    mpvSeek,
    mpvSetVolume,
    getItem,
  } from "$lib/jellyfinClient";

  let videoHost: HTMLDivElement;
  let error = $state("");
  let starting = $state(true);

  let paused = $state(false);
  let timePos = $state(0);
  let duration = $state(0);
  let volume = $state(100);

  let resizeObserver: ResizeObserver;
  let unlistenProperty: UnlistenFn | undefined;
  let seeking = false;

  function surfaceRect() {
    const rect = videoHost.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    return {
      x: Math.round(rect.left * dpr),
      y: Math.round(rect.top * dpr),
      width: Math.round(rect.width * dpr),
      height: Math.round(rect.height * dpr),
    };
  }

  onMount(async () => {
    const itemId = page.params.id!;

    unlistenProperty = await listen<{ name: string; value: unknown }>("mpv://property-change", (event) => {
      const { name, value } = event.payload;
      if (name === "time-pos" && !seeking && typeof value === "number") timePos = value;
      if (name === "duration" && typeof value === "number") duration = value;
      if (name === "pause" && typeof value === "boolean") paused = value;
    });

    try {
      const item = await getItem(itemId);
      const resumeTicks = item.UserData?.PlaybackPositionTicks ?? 0;
      const startSeconds = resumeTicks / 10_000_000;

      const rect = surfaceRect();
      await startPlayback(itemId, rect.x, rect.y, rect.width, rect.height, startSeconds);
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to start playback";
    } finally {
      starting = false;
    }

    resizeObserver = new ResizeObserver(() => {
      const rect = surfaceRect();
      resizeVideoSurface(rect.x, rect.y, rect.width, rect.height).catch(() => {});
    });
    resizeObserver.observe(videoHost);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    unlistenProperty?.();
    stopPlayback().catch(() => {});
  });

  function togglePause() {
    paused = !paused;
    mpvSetPause(paused).catch(() => {});
  }

  function handleSeekInput(event: Event) {
    seeking = true;
    timePos = Number((event.target as HTMLInputElement).value);
  }

  function handleSeekCommit(event: Event) {
    const seconds = Number((event.target as HTMLInputElement).value);
    mpvSeek(seconds).catch(() => {});
    seeking = false;
  }

  function handleVolumeInput(event: Event) {
    volume = Number((event.target as HTMLInputElement).value);
    mpvSetVolume(volume).catch(() => {});
  }

  function formatTime(seconds: number): string {
    if (!Number.isFinite(seconds)) return "0:00";
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function handleBack() {
    await stopPlayback().catch(() => {});
    goto(`/item/${page.params.id}`);
  }
</script>

<main>
  <div class="video-area" bind:this={videoHost}>
    {#if starting}
      <p class="dim">Starting playback&hellip;</p>
    {:else if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  <div class="controls">
    <button class="icon" onclick={handleBack} title="Back">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
    </button>
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
    <button class="icon" title="Audio track (coming soon)" disabled>
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><line x1="6" y1="16" x2="6" y2="10"></line><line x1="12" y1="19" x2="12" y2="6"></line><line x1="18" y1="15" x2="18" y2="12"></line></svg>
    </button>
    <button class="icon cc" title="Captions (coming soon)" disabled>CC</button>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: black;
  }

  .video-area {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 0;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    background: var(--bg-elevated);
    border-top: 1px solid var(--border);
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
  }

  .icon:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .icon:disabled {
    color: var(--text-dim);
    cursor: default;
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
  }

  .seek {
    flex: 1;
    accent-color: var(--accent);
  }

  .volume {
    width: 100px;
    accent-color: var(--accent);
  }

  .dim {
    color: var(--text-dim);
  }

  .error {
    color: var(--danger);
  }
</style>

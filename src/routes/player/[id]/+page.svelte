<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listen, emit, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { startPlayback, stopPlayback, getItem, toggleMainFullscreen } from "$lib/jellyfinClient";

  let error = $state("");
  let starting = $state(true);
  let unlistenNavigateBack: UnlistenFn | undefined;
  let unlistenPlay: UnlistenFn | undefined;

  // The HUD overlay window is deliberately non-focusable (see
  // create_hud_window in the Rust backend), so every keypress during
  // playback lands here, in the main window -- not on the HUD that actually
  // owns the playback state (position, volume, open panels, skip segments).
  // Rather than duplicate all of that here, raw keys are forwarded to the
  // HUD over an event and it decides what they mean. Escape-out-of-
  // fullscreen is also handled locally as a belt-and-braces fallback.
  async function handleKeydown(event: KeyboardEvent) {
    const modifier = event.ctrlKey || event.altKey || event.metaKey;
    if (!modifier) {
      const navKeys = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", " "];
      if (navKeys.includes(event.key)) event.preventDefault();
      emit("player://keydown", { key: event.key, shift: event.shiftKey }).catch(() => {});
    }
    if (event.key !== "Escape") return;
    if (await getCurrentWindow().isFullscreen().catch(() => false)) {
      toggleMainFullscreen().catch(() => {});
    }
  }

  async function play(itemId: string) {
    starting = true;
    error = "";
    try {
      const item = await getItem(itemId);
      const resumeTicks = item.UserData?.PlaybackPositionTicks ?? 0;
      const startSeconds = resumeTicks / 10_000_000;
      await startPlayback(itemId, startSeconds);
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to start playback";
    } finally {
      starting = false;
    }
  }

  onMount(async () => {
    // The HUD (top bar, playback controls, track pickers) lives in a
    // separate always-on-top overlay window -- see create_hud_window's doc
    // comment in the Rust backend for why -- so its "back" button can't just
    // call this window's own `goto()`. It goes through a command instead,
    // which emits this event back to "main" once playback has actually
    // stopped.
    unlistenNavigateBack = await listen<string>("player://navigate-back", (event) => {
      goto(`/item/${event.payload}`);
    });

    // Same deal for switching to another item mid-playback (autoplay-next,
    // the HUD's episode picker): the HUD asks, this window navigates, and
    // the `$effect` below restarts playback for the new route param.
    unlistenPlay = await listen<string>("player://play", (event) => {
      goto(`/player/${event.payload}`, { replaceState: true });
    });

    window.addEventListener("keydown", handleKeydown);
  });

  // Runs on mount and again whenever the route's item id changes -- this
  // component instance is reused across /player/A -> /player/B, so a plain
  // onMount would only ever start the first item. `start_playback` tears
  // the previous playback down itself before starting the new one.
  $effect(() => {
    play(page.params.id!);
  });

  onDestroy(() => {
    unlistenNavigateBack?.();
    unlistenPlay?.();
    window.removeEventListener("keydown", handleKeydown);
    stopPlayback().catch(() => {});
  });
</script>

<main>
  {#if starting}
    <p class="dim">Starting playback&hellip;</p>
  {:else if error}
    <p class="error">{error}</p>
  {/if}
</main>

<style>
  main {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    background: black;
  }

  .dim {
    color: var(--text-dim);
  }

  .error {
    color: var(--danger);
  }
</style>

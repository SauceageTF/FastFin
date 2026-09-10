<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { startPlayback, stopPlayback, getItem, toggleMainFullscreen } from "$lib/jellyfinClient";

  let error = $state("");
  let starting = $state(true);
  let unlistenNavigateBack: UnlistenFn | undefined;

  // A second, independent path out of fullscreen alongside the HUD window's
  // own Escape handler: after any fullscreen toggle this window explicitly
  // regains keyboard focus (see toggle_main_fullscreen in the Rust backend),
  // so it's often this window, not the HUD's, that actually receives the
  // keydown.
  async function handleKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    if (await getCurrentWindow().isFullscreen().catch(() => false)) {
      toggleMainFullscreen().catch(() => {});
    }
  }

  onMount(async () => {
    const itemId = page.params.id!;

    // The HUD (top bar, playback controls, track pickers) lives in a
    // separate always-on-top overlay window -- see create_hud_window's doc
    // comment in the Rust backend for why -- so its "back" button can't just
    // call this window's own `goto()`. It goes through a command instead,
    // which emits this event back to "main" once playback has actually
    // stopped.
    unlistenNavigateBack = await listen<string>("player://navigate-back", (event) => {
      goto(`/item/${event.payload}`);
    });

    window.addEventListener("keydown", handleKeydown);

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
  });

  onDestroy(() => {
    unlistenNavigateBack?.();
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

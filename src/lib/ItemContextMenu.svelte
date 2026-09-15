<script lang="ts">
  import { itemMenu, closeItemMenu } from "$lib/contextMenu";
  import { setPlayed } from "$lib/jellyfinClient";

  let busy = $state(false);
  let error = $state("");

  // Keep the menu inside the viewport when opened near the right/bottom edge.
  const MENU_W = 220;
  const MENU_H = 88;

  function position(x: number, y: number) {
    const left = Math.min(x, window.innerWidth - MENU_W - 8);
    const top = Math.min(y, window.innerHeight - MENU_H - 8);
    return `left:${left}px;top:${top}px`;
  }

  async function toggleWatched() {
    const state = $itemMenu;
    if (!state || busy) return;
    const item = state.item;
    const nextPlayed = !(item.UserData?.Played ?? false);
    busy = true;
    error = "";
    try {
      await setPlayed(item.Id, nextPlayed);
      // Mirror what Jellyfin does server-side so the card updates in place.
      if (!item.UserData) item.UserData = { PlaybackPositionTicks: 0, PlayedPercentage: null, Played: null };
      item.UserData.Played = nextPlayed;
      item.UserData.PlaybackPositionTicks = 0;
      item.UserData.PlayedPercentage = null;
      closeItemMenu();
    } catch (e) {
      error = typeof e === "string" ? e : "Couldn't update";
    } finally {
      busy = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && $itemMenu) closeItemMenu();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $itemMenu}
  <div class="scrim" onclick={closeItemMenu} oncontextmenu={(e) => { e.preventDefault(); closeItemMenu(); }} role="presentation"></div>
  <div class="menu" style={position($itemMenu.x, $itemMenu.y)} role="menu">
    <div class="label">{$itemMenu.item.Name}</div>
    <button class="entry" onclick={toggleWatched} disabled={busy} role="menuitem">
      {#if $itemMenu.item.UserData?.Played}
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"></path><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"></path><line x1="1" y1="1" x2="23" y2="23"></line></svg>
        Mark as unwatched
      {:else}
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
        Mark as watched
      {/if}
    </button>
    {#if error}<div class="error">{error}</div>{/if}
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
  }

  .menu {
    position: fixed;
    z-index: 41;
    min-width: 220px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 6px;
    box-shadow: 0 16px 32px rgba(0, 0, 0, 0.55);
  }

  .label {
    padding: 6px 10px 8px;
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 260px;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: 0.875rem;
    font-weight: 600;
    text-align: left;
  }

  .entry:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .entry:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .error {
    padding: 4px 10px 6px;
    font-size: 0.75rem;
    color: var(--danger);
  }
</style>

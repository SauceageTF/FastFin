<script lang="ts">
  import type { Item } from "$lib/types";

  // Watched-state decoration for any card: a checkmark badge once an item
  // is fully watched, a progress bar along the bottom while it's partway
  // through. Overlays the card's image, so the parent needs
  // `position: relative` and `overflow: hidden` to clip the bar's corners.
  let { item }: { item: Item } = $props();

  let played = $derived(item.UserData?.Played ?? false);
  let percent = $derived(!played && item.UserData?.PlayedPercentage ? item.UserData.PlayedPercentage : 0);
</script>

{#if played}
  <span class="badge" title="Watched">
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
  </span>
{:else if percent > 0}
  <div class="track">
    <div class="fill" style={`width:${Math.min(100, percent)}%`}></div>
  </div>
{/if}

<style>
  .badge {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 999px;
    background: var(--accent);
    color: #141018;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  }

  .track {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    background: rgba(255, 255, 255, 0.25);
  }

  .fill {
    height: 100%;
    background: var(--accent);
  }
</style>

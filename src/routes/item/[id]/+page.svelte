<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import Header from "$lib/Header.svelte";
  import { getItem, getImageUrl, getBackdropUrl, getSeasons } from "$lib/jellyfinClient";
  import { backdropSourceId, type Item } from "$lib/types";

  let item = $state<Item | null>(null);
  let imageUrl = $state("");
  let backdropUrl = $state("");
  let seasons = $state<Item[]>([]);
  let error = $state("");
  let loading = $state(true);

  function formatRuntime(ticks: number | null): string {
    if (!ticks) return "";
    const totalMinutes = Math.round(ticks / 10_000_000 / 60);
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;
    return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  }

  onMount(async () => {
    const itemId = page.params.id!;
    try {
      item = await getItem(itemId);
      const backdropId = backdropSourceId(item);
      [imageUrl, backdropUrl] = await Promise.all([
        getImageUrl(itemId),
        backdropId ? getBackdropUrl(backdropId).catch(() => "") : Promise.resolve(""),
      ]);
      if (item.Type === "Series") {
        seasons = await getSeasons(itemId);
      }
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load item";
    } finally {
      loading = false;
    }
  });
</script>

<Header />

<main>
  {#if loading}
    <p class="dim center">Loading&hellip;</p>
  {:else if error}
    <p class="error center">{error}</p>
  {:else if item}
    <section class="backdrop">
      {#if backdropUrl}
        <img src={backdropUrl} alt="" />
      {/if}
      <div class="backdrop-scrim"></div>
    </section>

    <section class="detail">
      <img class="poster" src={imageUrl} alt={item.Name} />
      <div class="info">
        <h1>{item.Name}</h1>
        <p class="meta">
          {#if item.ProductionYear}<span>{item.ProductionYear}</span>{/if}
          {#if item.RunTimeTicks}<span>{formatRuntime(item.RunTimeTicks)}</span>{/if}
        </p>
        {#if item.Overview}
          <p class="overview">{item.Overview}</p>
        {/if}
        {#if item.Type !== "Series"}
          <a class="play" href={`/player/${item.Id}`}>
            &#9658; {item.UserData?.PlaybackPositionTicks ? "Resume" : "Play"}
          </a>
          {#if item.UserData?.PlayedPercentage}
            <div class="progress-track">
              <div class="progress-fill" style={`width: ${item.UserData.PlayedPercentage}%`}></div>
            </div>
          {/if}
        {/if}
      </div>
    </section>

    {#if item.Type === "Series"}
      <section class="seasons-section">
        <h2>Seasons</h2>
        {#if seasons.length === 0}
          <p class="dim">No seasons found.</p>
        {:else}
          <div class="seasons">
            {#each seasons as season (season.Id)}
              <a class="season" href={`/item/${item.Id}/season/${season.Id}`}>
                {season.Name}
              </a>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  {/if}
</main>

<style>
  main {
    padding-bottom: 3rem;
  }

  .center {
    text-align: center;
    padding: 3rem 0;
  }

  .backdrop {
    position: relative;
    width: 100%;
    height: 320px;
    overflow: hidden;
  }

  .backdrop img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .backdrop-scrim {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(10, 10, 14, 0.3) 0%, rgba(10, 10, 14, 0.1) 50%, var(--bg) 100%);
  }

  .detail {
    position: relative;
    margin-top: -140px;
    padding: 0 40px;
    display: flex;
    gap: 2rem;
  }

  .poster {
    width: 200px;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 10px;
    border: 1px solid var(--border);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
    flex-shrink: 0;
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding-top: 2.25rem;
  }

  h1 {
    font-family: var(--font-display);
    font-weight: 800;
    margin: 0;
  }

  .seasons-section h2 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }

  .seasons-section {
    padding: 2rem 40px 0;
  }

  .meta {
    display: flex;
    gap: 0.75rem;
    color: var(--text-dim);
    font-size: 0.9rem;
    margin: 0;
  }

  .overview {
    color: var(--text);
    line-height: 1.5;
    max-width: 60ch;
  }

  .play {
    align-self: flex-start;
    margin-top: 0.5rem;
    background: var(--text);
    color: #141018;
    text-decoration: none;
    border-radius: 8px;
    padding: 0.65rem 1.5rem;
    font-weight: 700;
  }

  .play:hover {
    opacity: 0.85;
  }

  .progress-track {
    width: 200px;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
  }

  .seasons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .season {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.6rem 1.1rem;
    color: var(--text);
    text-decoration: none;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .season:hover {
    border-color: var(--accent);
  }

  .dim {
    color: var(--text-dim);
  }

  .error {
    color: var(--danger);
  }
</style>

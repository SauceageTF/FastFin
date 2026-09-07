<script lang="ts">
  import { onMount } from "svelte";
  import Header from "$lib/Header.svelte";
  import Carousel from "$lib/Carousel.svelte";
  import { getLibraries, getItems, getResume, getImageUrl, getBackdropUrl } from "$lib/jellyfinClient";
  import { backdropSourceId, type Library, type Item } from "$lib/types";

  type Row = { library: Library; items: Item[] };

  let continueWatching = $state<Item[]>([]);
  let rows = $state<Row[]>([]);
  let images = $state<Record<string, string>>({});
  let featured = $state<Item | null>(null);
  let featuredBackdrop = $state("");
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
    try {
      const [libraries, resumeItems] = await Promise.all([getLibraries(), getResume().catch(() => [])]);
      continueWatching = resumeItems;

      rows = await Promise.all(
        libraries.map(async (library) => {
          const items = await getItems(library.Id);
          return { library, items: items.slice(0, 15) };
        }),
      );

      featured = continueWatching[0] ?? rows.find((r) => r.items.length > 0)?.items[0] ?? null;
      if (featured) {
        const backdropId = backdropSourceId(featured);
        if (backdropId) {
          featuredBackdrop = await getBackdropUrl(backdropId);
        }
      }

      const allItems = [...continueWatching, ...rows.flatMap((r) => r.items)];
      const entries = await Promise.all(
        allItems.map(async (item) => [item.Id, await getImageUrl(item.Id)] as const),
      );
      images = Object.fromEntries(entries);
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load your library";
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
  {:else}
    {#if featured}
      <section class="hero">
        {#if featuredBackdrop}
          <img src={featuredBackdrop} alt="" class="hero-image" />
        {/if}
        <div class="hero-scrim"></div>
        <div class="hero-scrim-side"></div>
        <div class="hero-content">
          <h1>{featured.Name}</h1>
          <p class="hero-meta">
            {#if featured.SeriesName}<span>{featured.SeriesName}</span>{/if}
            {#if featured.ProductionYear}<span>{featured.ProductionYear}</span>{/if}
            {#if featured.RunTimeTicks}<span>{formatRuntime(featured.RunTimeTicks)}</span>{/if}
          </p>
          {#if featured.Overview}
            <p class="hero-overview">{featured.Overview}</p>
          {/if}
          <div class="hero-actions">
            {#if featured.Type === "Series"}
              <a class="btn primary" href={`/item/${featured.Id}`}>View Episodes</a>
            {:else}
              <a class="btn primary" href={`/player/${featured.Id}`}>&#9658; Play</a>
            {/if}
            <a class="btn secondary" href={`/item/${featured.Id}`}>More Info</a>
          </div>
        </div>
      </section>
    {/if}

    <div class="rows">
      {#if continueWatching.length > 0}
        <section class="row">
          <h2>Continue Watching</h2>
          <Carousel>
            {#each continueWatching as item (item.Id)}
              <a class="wide-card" href={`/player/${item.Id}`}>
                <div class="wide-thumb">
                  <img src={images[item.Id]} alt={item.Name} loading="lazy" />
                  {#if item.UserData?.PlayedPercentage}
                    <div class="progress-track">
                      <div class="progress-fill" style={`width:${item.UserData.PlayedPercentage}%`}></div>
                    </div>
                  {/if}
                </div>
                <span class="card-title">
                  {item.SeriesName ?? item.Name}
                  {#if item.SeriesName}<span class="dim"> &middot; {item.Name}</span>{/if}
                </span>
              </a>
            {/each}
          </Carousel>
        </section>
      {/if}

      {#each rows as row (row.library.Id)}
        {#if row.items.length > 0}
          <section class="row">
            <div class="row-header">
              <h2>{row.library.Name}</h2>
              <a class="see-all" href={`/library/${row.library.Id}`}>See all</a>
            </div>
            <Carousel>
              {#each row.items as item (item.Id)}
                <a class="poster-card" href={`/item/${item.Id}`}>
                  <img src={images[item.Id]} alt={item.Name} loading="lazy" />
                  <span class="card-title">{item.Name}</span>
                </a>
              {/each}
            </Carousel>
          </section>
        {/if}
      {/each}
    </div>
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

  .hero {
    position: relative;
    width: 100%;
    height: 82vh;
    min-height: 560px;
    overflow: hidden;
  }

  .hero-image {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .hero-scrim {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 60%;
    background: linear-gradient(180deg, transparent, var(--bg) 92%);
  }

  .hero-scrim-side {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 55%;
    background: linear-gradient(90deg, rgba(10, 10, 14, 0.55), transparent);
  }

  .hero-content {
    position: absolute;
    left: 40px;
    bottom: 72px;
    max-width: 720px;
  }

  .hero-content h1 {
    font-family: var(--font-display);
    font-weight: 800;
    font-size: 64px;
    line-height: 1.02;
    margin: 0 0 20px;
  }

  .hero-meta {
    display: flex;
    gap: 14px;
    color: var(--text-dim);
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 16px;
  }

  .hero-overview {
    font-size: 16px;
    line-height: 1.65;
    color: #c7c5d0;
    margin: 0 0 28px;
    max-width: 620px;
  }

  .hero-actions {
    display: flex;
    gap: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-radius: 6px;
    padding: 12px 24px;
    font-size: 14px;
    font-weight: 700;
    text-decoration: none;
  }

  .btn.primary {
    background: var(--text);
    color: #141018;
  }

  .btn.secondary {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text);
    border: 1px solid rgba(255, 255, 255, 0.18);
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 32px;
    padding: 0 40px;
    margin-top: 32px;
    position: relative;
  }

  .row-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  .row h2 {
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 700;
    margin: 0 0 14px;
  }

  .see-all {
    font-size: 13px;
    color: var(--text-dim);
    text-decoration: none;
  }

  .see-all:hover {
    color: var(--text);
  }

  .poster-card {
    flex-shrink: 0;
    width: 230px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    text-decoration: none;
    color: var(--text);
  }

  .poster-card img {
    width: 230px;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  .poster-card:hover img {
    outline: 2px solid var(--accent);
  }

  .wide-card {
    flex-shrink: 0;
    width: 340px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    text-decoration: none;
    color: var(--text);
  }

  .wide-thumb {
    position: relative;
    width: 340px;
    aspect-ratio: 16 / 9;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  .wide-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .wide-card:hover .wide-thumb {
    outline: 2px solid var(--accent);
  }

  .progress-track {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    background: rgba(255, 255, 255, 0.2);
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
  }

  .card-title {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dim {
    color: var(--text-dim);
    font-weight: 400;
  }

  .error {
    color: var(--danger);
  }
</style>

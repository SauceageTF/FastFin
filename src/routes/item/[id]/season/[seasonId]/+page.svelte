<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import Header from "$lib/Header.svelte";
  import { getEpisodes, getImageUrl } from "$lib/jellyfinClient";
  import type { Item } from "$lib/types";

  let episodes = $state<Item[] | null>(null);
  let images = $state<Record<string, string>>({});
  let error = $state("");

  function formatRuntime(ticks: number | null): string {
    if (!ticks) return "";
    const totalMinutes = Math.round(ticks / 10_000_000 / 60);
    return `${totalMinutes}m`;
  }

  onMount(async () => {
    const seriesId = page.params.id!;
    const seasonId = page.params.seasonId!;
    try {
      const loaded = await getEpisodes(seriesId, seasonId);
      episodes = loaded;
      const entries = await Promise.all(
        loaded.map(async (ep) => [ep.Id, await getImageUrl(ep.Id)] as const),
      );
      images = Object.fromEntries(entries);
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load episodes";
    }
  });
</script>

<Header />

<main>
  <a class="back" href={`/item/${page.params.id}`}>&larr; Back</a>

  {#if error}
    <p class="error">{error}</p>
  {:else if episodes === null}
    <div class="list">
      {#each Array(6) as _}
        <div class="row">
          <div class="skeleton row-thumb"></div>
          <div class="row-info">
            <div class="skeleton skel-line skel-title"></div>
            <div class="skeleton skel-line skel-overview"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if episodes.length === 0}
    <p class="dim">No episodes found.</p>
  {:else}
    <div class="list">
      {#each episodes as ep (ep.Id)}
        <a class="row" href={`/player/${ep.Id}`}>
          {#if images[ep.Id]}
            <img src={images[ep.Id]} alt={ep.Name} loading="lazy" />
          {:else}
            <div class="skeleton row-thumb"></div>
          {/if}
          <div class="row-info">
            <span class="title">
              {#if ep.IndexNumber}<span class="num">{ep.IndexNumber}.</span>{/if}
              {ep.Name}
            </span>
            {#if ep.RunTimeTicks}<span class="runtime">{formatRuntime(ep.RunTimeTicks)}</span>{/if}
            {#if ep.Overview}<p class="overview">{ep.Overview}</p>{/if}
          </div>
        </a>
      {/each}
    </div>
  {/if}
</main>

<style>
  main {
    padding: 88px 40px 3rem;
    max-width: 900px;
  }

  .back {
    display: inline-block;
    margin-bottom: 1.5rem;
    font-size: 0.9rem;
    text-decoration: none;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .row {
    display: flex;
    gap: 1rem;
    text-decoration: none;
    color: var(--text);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0.75rem;
  }

  .row:hover {
    background: var(--bg-hover);
  }

  .row img,
  .row-thumb {
    width: 160px;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    border-radius: 6px;
    flex-shrink: 0;
  }

  .skel-line {
    height: 1em;
  }

  .skel-title {
    width: 50%;
    height: 1.1em;
  }

  .skel-overview {
    width: 90%;
  }

  .row-info {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-width: 0;
  }

  .title {
    font-weight: 600;
  }

  .num {
    color: var(--text-dim);
    font-weight: 400;
    margin-right: 0.25rem;
  }

  .runtime {
    font-size: 0.8rem;
    color: var(--text-dim);
  }

  .overview {
    margin: 0;
    font-size: 0.85rem;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .dim {
    color: var(--text-dim);
  }

  .error {
    color: var(--danger);
  }
</style>

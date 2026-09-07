<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import Header from "$lib/Header.svelte";
  import { getEpisodes, getImageUrl } from "$lib/jellyfinClient";
  import type { Item } from "$lib/types";

  let episodes = $state<Item[]>([]);
  let images = $state<Record<string, string>>({});
  let error = $state("");
  let loading = $state(true);

  function formatRuntime(ticks: number | null): string {
    if (!ticks) return "";
    const totalMinutes = Math.round(ticks / 10_000_000 / 60);
    return `${totalMinutes}m`;
  }

  onMount(async () => {
    const seriesId = page.params.id!;
    const seasonId = page.params.seasonId!;
    try {
      episodes = await getEpisodes(seriesId, seasonId);
      const entries = await Promise.all(
        episodes.map(async (ep) => [ep.Id, await getImageUrl(ep.Id)] as const),
      );
      images = Object.fromEntries(entries);
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load episodes";
    } finally {
      loading = false;
    }
  });
</script>

<Header />

<main>
  <a class="back" href={`/item/${page.params.id}`}>&larr; Back</a>

  {#if loading}
    <p class="dim">Loading&hellip;</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if episodes.length === 0}
    <p class="dim">No episodes found.</p>
  {:else}
    <div class="list">
      {#each episodes as ep (ep.Id)}
        <a class="row" href={`/player/${ep.Id}`}>
          <img src={images[ep.Id]} alt={ep.Name} loading="lazy" />
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

  .row img {
    width: 160px;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    border-radius: 6px;
    flex-shrink: 0;
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

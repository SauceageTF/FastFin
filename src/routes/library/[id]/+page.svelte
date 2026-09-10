<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import Header from "$lib/Header.svelte";
  import { getItems, getImageUrl } from "$lib/jellyfinClient";
  import type { Item } from "$lib/types";

  let items = $state<Item[] | null>(null);
  let images = $state<Record<string, string>>({});
  let error = $state("");

  onMount(async () => {
    const libraryId = page.params.id!;
    try {
      const loaded = await getItems(libraryId);
      items = loaded;
      // Render the grid as soon as the item list itself is known -- images
      // fill in individually right after, rather than gating the whole
      // grid behind every single poster finishing.
      const entries = await Promise.all(
        loaded.map(async (item) => [item.Id, await getImageUrl(item.Id)] as const),
      );
      images = Object.fromEntries(entries);
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load items";
    }
  });
</script>

<Header />

<main>
  <a class="back" href="/library">&larr; Libraries</a>

  {#if error}
    <p class="error">{error}</p>
  {:else if items === null}
    <div class="grid">
      {#each Array(18) as _}
        <div class="poster">
          <div class="skeleton poster-thumb"></div>
        </div>
      {/each}
    </div>
  {:else if items.length === 0}
    <p class="dim">This library is empty.</p>
  {:else}
    <div class="grid">
      {#each items as item (item.Id)}
        <a class="poster" href={`/item/${item.Id}`}>
          {#if images[item.Id]}
            <img src={images[item.Id]} alt={item.Name} loading="lazy" />
          {:else}
            <div class="skeleton poster-thumb"></div>
          {/if}
          <span class="title">{item.Name}</span>
          {#if item.ProductionYear}
            <span class="year">{item.ProductionYear}</span>
          {/if}
        </a>
      {/each}
    </div>
  {/if}
</main>

<style>
  main {
    padding: 88px 40px 3rem;
  }

  .back {
    display: inline-block;
    margin-bottom: 1.5rem;
    font-size: 0.9rem;
    text-decoration: none;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1.25rem;
  }

  .poster {
    display: flex;
    flex-direction: column;
    text-decoration: none;
    color: var(--text);
  }

  .poster img,
  .poster-thumb {
    width: 100%;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  .poster:hover img {
    outline: 2px solid var(--accent);
  }

  .title {
    margin-top: 0.5rem;
    font-size: 0.85rem;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .year {
    font-size: 0.75rem;
    color: var(--text-dim);
  }

  .dim {
    color: var(--text-dim);
  }

  .error {
    color: var(--danger);
  }
</style>

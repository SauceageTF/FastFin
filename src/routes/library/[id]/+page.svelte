<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import Header from "$lib/Header.svelte";
  import WatchedOverlay from "$lib/WatchedOverlay.svelte";
  import { openItemMenu } from "$lib/contextMenu";
  import { getItems, getImageUrl } from "$lib/jellyfinClient";
  import { getLibrarySort, setLibrarySort, type LibrarySort, type SortKey } from "$lib/prefs";
  import type { Item } from "$lib/types";

  const SORT_OPTIONS: { key: SortKey; label: string }[] = [
    { key: "name", label: "Name" },
    { key: "added", label: "Date added" },
    { key: "year", label: "Release year" },
    { key: "rating", label: "Rating" },
    { key: "runtime", label: "Runtime" },
  ];

  let libraryId = $state("");
  let items = $state<Item[] | null>(null);
  let images = $state<Record<string, string>>({});
  let error = $state("");
  let sort = $state<LibrarySort>({ key: "name", descending: false });
  let genre = $state("");

  // Every distinct genre across the library, for the filter chips.
  let genres = $derived.by(() => {
    const set = new Set<string>();
    for (const item of items ?? []) for (const g of item.Genres ?? []) set.add(g);
    return [...set].sort((a, b) => a.localeCompare(b));
  });

  // Sorting is done client-side: the whole library is already loaded for
  // the grid, so re-sorting is instant and doesn't need another request.
  // Items missing the sort field (no rating, no year, ...) always sink to
  // the bottom regardless of direction, so the interesting end of the list
  // is never a wall of blanks.
  let visibleItems = $derived.by(() => {
    if (!items) return null;
    const filtered = genre ? items.filter((i) => i.Genres?.includes(genre)) : items;
    const dir = sort.descending ? -1 : 1;
    const value = (item: Item): number | string | null => {
      switch (sort.key) {
        case "name":
          return item.Name.toLowerCase();
        case "added":
          return item.DateCreated ? Date.parse(item.DateCreated) : null;
        case "year":
          return item.ProductionYear;
        case "rating":
          return item.CommunityRating;
        case "runtime":
          return item.RunTimeTicks;
      }
    };
    return [...filtered].sort((a, b) => {
      const va = value(a);
      const vb = value(b);
      if (va == null && vb == null) return 0;
      if (va == null) return 1;
      if (vb == null) return -1;
      if (va < vb) return -1 * dir;
      if (va > vb) return 1 * dir;
      return a.Name.localeCompare(b.Name);
    });
  });

  let movies = $derived((items ?? []).filter((i) => i.Type === "Movie"));

  function chooseSort(key: SortKey) {
    // Re-picking the current key flips direction; a new key starts in the
    // direction that makes sense for it (newest / best / longest first).
    if (sort.key === key) sort = { key, descending: !sort.descending };
    else sort = { key, descending: key !== "name" };
    setLibrarySort(libraryId, sort);
  }

  function toggleDirection() {
    sort = { ...sort, descending: !sort.descending };
    setLibrarySort(libraryId, sort);
  }

  function surpriseMe() {
    const pool = movies.length > 0 ? movies : [];
    if (pool.length === 0) return;
    const pick = pool[Math.floor(Math.random() * pool.length)];
    goto(`/player/${pick.Id}`);
  }

  async function load(id: string) {
    libraryId = id;
    items = null;
    images = {};
    error = "";
    genre = "";
    sort = getLibrarySort(id);
    try {
      const loaded = await getItems(id);
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
  }

  $effect(() => {
    load(page.params.id!);
  });
</script>

<Header />

<main>
  <div class="toolbar">
    <a class="back" href="/library">&larr; Libraries</a>

    {#if items && items.length > 0}
      <div class="sort">
        <span class="sort-label">Sort</span>
        {#each SORT_OPTIONS as option (option.key)}
          <button class="chip" class:active={sort.key === option.key} onclick={() => chooseSort(option.key)}>
            {option.label}
          </button>
        {/each}
        <button class="chip dir" onclick={toggleDirection} title={sort.descending ? "Descending" : "Ascending"} aria-label="Toggle sort direction">
          {#if sort.descending}
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><polyline points="19 12 12 19 5 12"></polyline></svg>
          {:else}
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline></svg>
          {/if}
        </button>
        {#if movies.length > 1}
          <button class="chip surprise" onclick={surpriseMe} title="Play a random movie">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 3 21 3 21 8"></polyline><line x1="4" y1="20" x2="21" y2="3"></line><polyline points="21 16 21 21 16 21"></polyline><line x1="15" y1="15" x2="21" y2="21"></line><line x1="4" y1="4" x2="9" y2="9"></line></svg>
            Surprise me
          </button>
        {/if}
      </div>
    {/if}
  </div>

  {#if genres.length > 1}
    <div class="genres">
      <button class="chip" class:active={genre === ""} onclick={() => (genre = "")}>All</button>
      {#each genres as g (g)}
        <button class="chip" class:active={genre === g} onclick={() => (genre = genre === g ? "" : g)}>{g}</button>
      {/each}
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {:else if visibleItems === null}
    <div class="grid">
      {#each Array(18) as _}
        <div class="poster">
          <div class="skeleton poster-thumb"></div>
        </div>
      {/each}
    </div>
  {:else if visibleItems.length === 0}
    <p class="dim">{genre ? `Nothing in ${genre}.` : "This library is empty."}</p>
  {:else}
    <div class="grid">
      {#each visibleItems as item (item.Id)}
        <a class="poster" href={`/item/${item.Id}`} oncontextmenu={(e) => openItemMenu(e, item)}>
          <div class="poster-frame">
            {#if images[item.Id]}
              <img src={images[item.Id]} alt={item.Name} loading="lazy" />
            {:else}
              <div class="skeleton poster-thumb"></div>
            {/if}
            <WatchedOverlay {item} />
          </div>
          <span class="title">{item.Name}</span>
          <span class="year">
            {#if sort.key === "rating" && item.CommunityRating}
              &#9733; {item.CommunityRating.toFixed(1)}
            {:else if sort.key === "runtime" && item.RunTimeTicks}
              {Math.round(item.RunTimeTicks / 600_000_000)}m
            {:else if item.ProductionYear}
              {item.ProductionYear}
            {/if}
          </span>
        </a>
      {/each}
    </div>
  {/if}
</main>

<style>
  main {
    padding: 88px 40px 3rem;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 0.75rem 1.5rem;
    margin-bottom: 1rem;
  }

  .back {
    display: inline-block;
    font-size: 0.9rem;
    text-decoration: none;
  }

  .sort {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }

  .sort-label {
    font-size: 0.75rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-dim);
    margin-right: 4px;
  }

  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 1.5rem;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-dim);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 999px;
  }

  .chip:hover {
    color: var(--text);
    border-color: var(--text-dim);
  }

  .chip.active {
    color: var(--text);
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .chip.dir {
    padding: 6px 8px;
    color: var(--text);
  }

  .chip.surprise {
    margin-left: 10px;
    color: var(--text);
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

  .poster-frame {
    position: relative;
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
  }

  .poster img,
  .poster-thumb {
    display: block;
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
    min-height: 1em;
  }

  .dim {
    color: var(--text-dim);
  }

  .error {
    color: var(--danger);
  }
</style>

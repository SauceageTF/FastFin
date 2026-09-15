<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { replaceState } from "$app/navigation";
  import Header from "$lib/Header.svelte";
  import { searchItems, getImageUrl } from "$lib/jellyfinClient";
  import type { Item } from "$lib/types";

  const DEBOUNCE_MS = 250;

  let term = $state("");
  // null = nothing searched yet (or term cleared); [] = searched, no hits.
  let results = $state<Item[] | null>(null);
  let images = $state<Record<string, string>>({});
  let loading = $state(false);
  let error = $state("");
  let inputEl: HTMLInputElement;

  let debounce: ReturnType<typeof setTimeout> | undefined;
  // Monotonic counter so a slow response for an older term can't overwrite
  // the results of a newer one that already came back.
  let requestSeq = 0;

  async function runSearch(query: string) {
    const seq = ++requestSeq;
    const trimmed = query.trim();
    if (!trimmed) {
      results = null;
      loading = false;
      error = "";
      return;
    }
    loading = true;
    error = "";
    try {
      const found = await searchItems(trimmed);
      if (seq !== requestSeq) return;
      results = found;
      // Same pattern as the library grid: show the results as soon as the
      // list is known and let posters fill in individually.
      const entries = await Promise.all(
        found.map(async (item) => [item.Id, await getImageUrl(item.Id)] as const),
      );
      if (seq !== requestSeq) return;
      images = { ...images, ...Object.fromEntries(entries) };
    } catch (e) {
      if (seq !== requestSeq) return;
      error = typeof e === "string" ? e : "Search failed";
    } finally {
      if (seq === requestSeq) loading = false;
    }
  }

  function handleInput() {
    if (debounce) clearTimeout(debounce);
    // Keep the term in the URL so coming back from an item page restores
    // the same search instead of an empty box.
    const url = new URL(page.url);
    if (term.trim()) url.searchParams.set("q", term);
    else url.searchParams.delete("q");
    replaceState(url, {});
    debounce = setTimeout(() => runSearch(term), DEBOUNCE_MS);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      if (debounce) clearTimeout(debounce);
      runSearch(term);
    } else if (event.key === "Escape") {
      // First Escape clears the box; a second one (empty box) leaves the page.
      if (!term) {
        history.back();
        return;
      }
      term = "";
      handleInput();
    }
  }

  function typeLabel(item: Item): string {
    return item.Type === "Series" ? "Series" : "Movie";
  }

  onMount(() => {
    term = page.url.searchParams.get("q") ?? "";
    inputEl?.focus();
    if (term) runSearch(term);
    return () => {
      if (debounce) clearTimeout(debounce);
    };
  });
</script>

<Header />

<main>
  <div class="search-box">
    <svg class="search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="11" cy="11" r="7"></circle>
      <line x1="21" y1="21" x2="16.2" y2="16.2"></line>
    </svg>
    <input
      bind:this={inputEl}
      bind:value={term}
      oninput={handleInput}
      onkeydown={handleKeydown}
      type="search"
      placeholder="Search movies and shows"
      autocomplete="off"
      spellcheck="false"
    />
    {#if term}
      <button class="clear" onclick={() => { term = ""; handleInput(); }} aria-label="Clear search">&times;</button>
    {/if}
  </div>

  {#if error}
    <p class="error">{error}</p>
  {:else if results === null}
    {#if loading}
      <div class="grid">
        {#each Array(12) as _}
          <div class="poster">
            <div class="skeleton poster-thumb"></div>
          </div>
        {/each}
      </div>
    {:else}
      <p class="dim">Type to search across your libraries.</p>
    {/if}
  {:else if results.length === 0}
    <p class="dim">No movies or shows match &ldquo;{term.trim()}&rdquo;.</p>
  {:else}
    <div class="grid" class:stale={loading}>
      {#each results as item (item.Id)}
        <a class="poster" href={`/item/${item.Id}`}>
          {#if images[item.Id]}
            <img src={images[item.Id]} alt={item.Name} loading="lazy" />
          {:else}
            <div class="skeleton poster-thumb"></div>
          {/if}
          <span class="title">{item.Name}</span>
          <span class="meta">
            {typeLabel(item)}{#if item.ProductionYear}&nbsp;&middot;&nbsp;{item.ProductionYear}{/if}
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

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
    max-width: 640px;
    margin: 0 auto 2rem;
  }

  .search-icon {
    position: absolute;
    left: 16px;
    color: var(--text-dim);
    pointer-events: none;
  }

  input {
    width: 100%;
    padding: 14px 44px 14px 48px;
    font-size: 1.05rem;
    color: var(--text);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    outline: none;
  }

  input::placeholder {
    color: var(--text-dim);
  }

  input:focus {
    border-color: var(--accent);
  }

  /* The browser's own clear button doubles up with ours below. */
  input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    position: absolute;
    right: 10px;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-dim);
    font-size: 1.3rem;
    line-height: 1;
  }

  .clear:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1.25rem;
    transition: opacity 0.15s;
  }

  /* Previous results stay on screen while the next query is in flight,
     just dimmed, so the grid doesn't flash empty on every keystroke. */
  .grid.stale {
    opacity: 0.5;
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

  .meta {
    font-size: 0.75rem;
    color: var(--text-dim);
  }

  .dim {
    color: var(--text-dim);
    text-align: center;
  }

  .error {
    color: var(--danger);
    text-align: center;
  }
</style>

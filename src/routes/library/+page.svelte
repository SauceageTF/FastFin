<script lang="ts">
  import { onMount } from "svelte";
  import Header from "$lib/Header.svelte";
  import Carousel from "$lib/Carousel.svelte";
  import WatchedOverlay from "$lib/WatchedOverlay.svelte";
  import { openItemMenu } from "$lib/contextMenu";
  import {
    getLibraries,
    getLatestItems,
    getResume,
    getNextUp,
    getImageUrl,
    getBackdropUrl,
    getThumbUrl,
    getLogoUrl,
  } from "$lib/jellyfinClient";
  import { backdropSourceId, hasLogo, episodeCode, type Library, type Item } from "$lib/types";

  // `items: null` means that row hasn't resolved yet (shown as a skeleton);
  // `[]` means it resolved but the library is empty (hidden entirely). Rows
  // are seeded in their final order up front and each fills in independently
  // as its own request finishes, instead of the whole page waiting on
  // whichever library happens to be slowest -- that "everything shows up at
  // once, whenever the last thing finishes" gate was the main thing making
  // the page feel like it was doing nothing for a noticeable stretch.
  type RowState = { library: Library; items: Item[] | null };

  let continueWatching = $state<Item[] | null>(null);
  let nextUp = $state<Item[] | null>(null);
  let rows = $state<RowState[]>([]);
  let images = $state<Record<string, string>>({});
  let wideImages = $state<Record<string, string>>({});
  let featured = $state<Item | null>(null);
  let featuredReady = $state(false);
  let featuredBackdrop = $state("");
  let featuredLogo = $state("");
  let error = $state("");

  function formatRuntime(ticks: number | null): string {
    if (!ticks) return "";
    const totalMinutes = Math.round(ticks / 10_000_000 / 60);
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;
    return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  }

  // getImageUrl/getBackdropUrl resolve almost instantly now (the session
  // info they need is cached after the first call -- see jellyfinClient.ts),
  // so awaiting *them* doesn't actually wait for the picture itself to
  // arrive over the network, just for its URL to be known. Actually
  // prioritizing the hero means waiting for its image to finish downloading
  // and decoding before anything else even starts requesting -- otherwise
  // the hero's (larger) backdrop and the row's (smaller) poster thumbnails
  // end up racing for bandwidth, and the smaller ones reliably win, which is
  // exactly the "row appears before the hero" effect this is fixing.
  function preloadImage(url: string): Promise<void> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => resolve();
      img.onerror = () => resolve();
      img.src = url;
      // Don't let a stalled/unreachable image hold up the rest of the page
      // forever.
      setTimeout(resolve, 4000);
    });
  }

  // The Continue Watching cards are 16:9. An episode's Primary image is
  // already a landscape still, but a movie's Primary is its portrait poster,
  // which looks wrong cropped into a wide card -- so movies use their
  // landscape Thumb art when they have one, else their backdrop, and only
  // fall back to the poster when neither exists.
  function wideCardImageUrl(item: Item): Promise<string> {
    if (item.Type === "Episode") return getImageUrl(item.Id);
    if (item.ImageTags?.Thumb) return getThumbUrl(item.Id);
    const backdropId = backdropSourceId(item);
    if (backdropId) return getBackdropUrl(backdropId, 800);
    return getImageUrl(item.Id);
  }

  async function resolveAndPreload(items: Item[], pickUrl: (item: Item) => Promise<string>) {
    const entries = await Promise.all(
      items.map(async (item) => [item.Id, await pickUrl(item)] as const),
    );
    await Promise.all(entries.map(([, url]) => preloadImage(url)));
    return Object.fromEntries(entries);
  }

  async function loadImagesFor(items: Item[]) {
    const loaded = await resolveAndPreload(items, (item) => getImageUrl(item.Id));
    images = { ...images, ...loaded };
  }

  // Kept separate from `images`: both maps are keyed by item id, and a movie
  // can sit in Continue Watching *and* a Recently Added row at once, where it
  // needs a landscape image in one and its poster in the other.
  async function loadWideImagesFor(items: Item[]) {
    const loaded = await resolveAndPreload(items, wideCardImageUrl);
    wideImages = { ...wideImages, ...loaded };
  }

  async function fetchRowItems(index: number): Promise<Item[]> {
    const library = rows[index].library;
    try {
      const items = await getLatestItems(library.Id);
      rows[index] = { library, items };
      return items;
    } catch {
      rows[index] = { library, items: [] };
      return [];
    }
  }

  async function loadRow(index: number) {
    const items = await fetchRowItems(index);
    if (items.length > 0) await loadImagesFor(items);
  }

  async function setFeatured(item: Item | null) {
    featured = item;
    featuredLogo = "";
    if (!item) {
      featuredReady = true;
      return;
    }
    const backdropId = backdropSourceId(item);
    const tasks: Promise<void>[] = [];
    if (backdropId) {
      tasks.push(
        getBackdropUrl(backdropId)
          .catch(() => "")
          .then(async (url) => {
            if (url) await preloadImage(url);
            featuredBackdrop = url;
          }),
      );
    }
    // The logo (wordmark) is a small transparent PNG, not the multi-hundred-
    // KB backdrop, so loading it alongside rather than after doesn't
    // meaningfully compete for bandwidth the way a second large image would.
    if (hasLogo(item)) {
      tasks.push(
        getLogoUrl(item.Id)
          .catch(() => "")
          .then(async (url) => {
            if (url) await preloadImage(url);
            featuredLogo = url;
          }),
      );
    }
    await Promise.all(tasks);
    featuredReady = true;
  }

  onMount(async () => {
    try {
      const [libraries, resumeItems] = await Promise.all([
        getLibraries(),
        getResume().catch(() => [] as Item[]),
      ]);

      rows = libraries.map((library) => ({ library, items: null }));

      // The hero takes strict top priority, then the first row -- each
      // fully finishes (image bytes and all, not just the URL) before the
      // next thing even starts requesting, all the way down to every other
      // row only being fetched once both of those are completely done.
      if (resumeItems.length > 0) {
        continueWatching = resumeItems;
        await setFeatured(resumeItems[0]);
        await loadWideImagesFor(resumeItems);
      } else {
        continueWatching = [];
        if (rows.length > 0) {
          const items = await fetchRowItems(0);
          await setFeatured(items[0] ?? null);
          if (items.length > 0) await loadImagesFor(items);
        } else {
          await setFeatured(null);
        }
      }

      rows.forEach((row, i) => {
        if (row.items === null) loadRow(i);
      });

      // Next Up (the next unwatched episode of every show in progress) is
      // distinct from Continue Watching (half-finished items) but can
      // overlap with it -- a half-watched episode is also that show's next
      // unwatched one -- so anything already in the row above is dropped.
      getNextUp()
        .then(async (items) => {
          const seen = new Set((continueWatching ?? []).map((i) => i.Id));
          const fresh = items.filter((i) => !seen.has(i.Id));
          nextUp = fresh;
          if (fresh.length > 0) await loadWideImagesFor(fresh);
        })
        .catch(() => (nextUp = []));
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load your library";
    }
  });
</script>

<Header />

<main>
  {#if error}
    <p class="error center">{error}</p>
  {:else}
    {#if !featuredReady}
      <section class="hero skeleton-hero">
        <div class="skeleton hero-fill"></div>
      </section>
    {:else if featured}
      <section class="hero">
        {#if featuredBackdrop}
          <img src={featuredBackdrop} alt="" class="hero-image" />
        {/if}
        <div class="hero-scrim"></div>
        <div class="hero-scrim-side"></div>
        <div class="hero-content">
          {#if featuredLogo}
            <img class="hero-logo" src={featuredLogo} alt={featured.Name} />
          {:else}
            <h1>{featured.Name}</h1>
          {/if}
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
      {#if continueWatching === null}
        <section class="row">
          <div class="skeleton skeleton-title"></div>
          <div class="skeleton-cards">
            {#each Array(6) as _}
              <div class="skeleton wide-thumb"></div>
            {/each}
          </div>
        </section>
      {:else if continueWatching.length > 0}
        <section class="row">
          <h2>Continue Watching</h2>
          <Carousel>
            {#each continueWatching as item (item.Id)}
              <a class="wide-card" href={`/player/${item.Id}`} oncontextmenu={(e) => openItemMenu(e, item)}>
                <div class="wide-thumb">
                  {#if wideImages[item.Id]}
                    <img src={wideImages[item.Id]} alt={item.Name} loading="lazy" />
                  {/if}
                  <WatchedOverlay {item} />
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

      {#if nextUp && nextUp.length > 0}
        <section class="row">
          <h2>Next Up</h2>
          <Carousel>
            {#each nextUp as item (item.Id)}
              <a class="wide-card" href={`/player/${item.Id}`} oncontextmenu={(e) => openItemMenu(e, item)}>
                <div class="wide-thumb">
                  {#if wideImages[item.Id]}
                    <img src={wideImages[item.Id]} alt={item.Name} loading="lazy" />
                  {/if}
                </div>
                <span class="card-title">
                  {item.SeriesName ?? item.Name}
                  {#if item.SeriesName}<span class="dim"> &middot; {episodeCode(item)} {item.Name}</span>{/if}
                </span>
              </a>
            {/each}
          </Carousel>
        </section>
      {/if}

      {#each rows as row (row.library.Id)}
        {#if row.items === null}
          <section class="row">
            <div class="skeleton skeleton-title"></div>
            <div class="skeleton-cards">
              {#each Array(6) as _}
                <div class="skeleton poster-thumb"></div>
              {/each}
            </div>
          </section>
        {:else if row.items.length > 0}
          <section class="row">
            <div class="row-header">
              <h2>Recently Added in {row.library.Name}</h2>
              <a class="see-all" href={`/library/${row.library.Id}`}>See all</a>
            </div>
            <Carousel>
              {#each row.items as item (item.Id)}
                <a class="poster-card" href={`/item/${item.Id}`} oncontextmenu={(e) => openItemMenu(e, item)}>
                  <div class="poster-frame">
                    {#if images[item.Id]}
                      <img src={images[item.Id]} alt={item.Name} loading="lazy" />
                    {:else}
                      <div class="skeleton poster-thumb"></div>
                    {/if}
                    <WatchedOverlay {item} />
                  </div>
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

  .skeleton-hero {
    padding: 0 40px;
    box-sizing: border-box;
  }

  .hero-fill {
    position: absolute;
    inset: 0;
    border-radius: 0;
  }

  /* Backdrops are 16:9 but the hero is much wider than that (full width,
     82vh tall -- ~2.2:1 on a 16:9 window), so stretching the image to cover
     it used to crop ~18% of its height, and since `cover` crops from the
     center that took heads off the top. Instead the image keeps its natural
     aspect at full hero height and sits flush right, so nothing is cropped
     on any window at least as wide as 16:9; the strip it leaves on the left
     is page background, which is where the title/overview go anyway, and
     the mask below fades the image's left edge into it so there's no seam.
     Only a narrower-than-16:9 window (rare for a desktop app) makes the
     image wider than the hero -- `max-width` then clamps it and `cover`
     crops, anchored to the top so it's the bottom (already fading into the
     page) that gives, never faces. */
  .hero-image {
    position: absolute;
    top: 0;
    right: 0;
    height: 100%;
    width: auto;
    max-width: 100%;
    object-fit: cover;
    object-position: center top;
    mask-image: linear-gradient(90deg, transparent, #000 40%);
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 40%);
  }

  .hero-scrim {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 60%;
    background: linear-gradient(180deg, transparent, var(--bg) 92%);
  }

  /* Text legibility only -- the image's own left-edge fade (see
     .hero-image) is what does the blending into the page background. */
  .hero-scrim-side {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 55%;
    background: linear-gradient(90deg, rgba(10, 10, 14, 0.7), transparent);
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
    font-size: 4rem;
    line-height: 1.02;
    margin: 0 0 20px;
  }

  .hero-logo {
    display: block;
    max-width: 360px;
    max-height: 140px;
    width: auto;
    height: auto;
    object-fit: contain;
    margin: 0 0 20px;
    filter: drop-shadow(0 4px 16px rgba(0, 0, 0, 0.5));
  }

  .hero-meta {
    display: flex;
    gap: 14px;
    color: var(--text-dim);
    font-size: 0.875rem;
    font-weight: 600;
    margin: 0 0 16px;
  }

  .hero-overview {
    font-size: 1rem;
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
    font-size: 0.875rem;
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
    font-size: 1.125rem;
    font-weight: 700;
    margin: 0 0 14px;
  }

  .skeleton-title {
    width: 220px;
    height: 18px;
    margin-bottom: 14px;
  }

  .skeleton-cards {
    display: flex;
    gap: 12px;
    overflow: hidden;
  }

  .see-all {
    font-size: 0.8125rem;
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

  .poster-frame {
    position: relative;
    width: 230px;
    border-radius: 8px;
    overflow: hidden;
  }

  .poster-card img,
  .poster-thumb {
    display: block;
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

  .card-title {
    font-size: 0.8125rem;
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

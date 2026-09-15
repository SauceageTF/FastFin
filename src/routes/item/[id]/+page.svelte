<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import Header from "$lib/Header.svelte";
  import Carousel from "$lib/Carousel.svelte";
  import {
    getItem,
    getImageUrl,
    getBackdropUrl,
    getSeasons,
    getEpisodes,
    getSeriesEpisodes,
    getNextUp,
    getSimilarItems,
  } from "$lib/jellyfinClient";
  import { backdropSourceId, episodeCode, type Item } from "$lib/types";
  import WatchedOverlay from "$lib/WatchedOverlay.svelte";
  import { openItemMenu } from "$lib/contextMenu";

  let item = $state<Item | null>(null);
  let imageUrl = $state("");
  let backdropUrl = $state("");
  // `null` means "still loading" (shown as a skeleton), distinct from an
  // empty array (genuinely nothing to show, section hidden). These sections
  // used to be gated behind the same `loading` flag as the core item info
  // above, so the whole page -- backdrop, poster, title, overview, all of
  // which are ready the moment `getItem` resolves -- sat behind a skeleton
  // for as long as the *slowest* of these secondary sections took. Now
  // `loading` only covers the core info, and each of these loads
  // independently afterward.
  let seasons = $state<Item[] | null>(null);
  let seasonImages = $state<Record<string, string>>({});
  let nextEpisodes = $state<Item[] | null>(null);
  let nextEpisodeImages = $state<Record<string, string>>({});
  let similarItems = $state<Item[] | null>(null);
  let similarImages = $state<Record<string, string>>({});
  let error = $state("");
  let loading = $state(true);
  // The series' "Continue" target: the next unwatched episode (or S1E1 for
  // a show that's never been started). null while loading, undefined once
  // known to have nothing (fully watched show).
  let nextUpEpisode = $state<Item | null | undefined>(null);
  let shuffling = $state(false);
  let shuffleError = $state("");

  // Picks a random episode across every season and jumps straight into
  // playback. Specials (season 0) are left out -- they're rarely what
  // someone wants from "play me anything" -- unless the show has nothing
  // else. The episode list is fetched on click rather than up front since
  // it can be hundreds of items for a long-running show and only this
  // button ever needs it.
  async function playRandomEpisode(seriesId: string) {
    if (shuffling) return;
    shuffling = true;
    shuffleError = "";
    try {
      const all = await getSeriesEpisodes(seriesId);
      const regular = all.filter((ep) => (ep.ParentIndexNumber ?? 1) !== 0);
      const pool = regular.length > 0 ? regular : all;
      if (pool.length === 0) {
        shuffleError = "No episodes to pick from.";
        return;
      }
      const pick = pool[Math.floor(Math.random() * pool.length)];
      await goto(`/player/${pick.Id}`);
    } catch (e) {
      shuffleError = typeof e === "string" ? e : "Couldn't pick an episode";
    } finally {
      shuffling = false;
    }
  }

  function formatRuntime(ticks: number | null): string {
    if (!ticks) return "";
    const totalMinutes = Math.round(ticks / 10_000_000 / 60);
    const hours = Math.floor(totalMinutes / 60);
    const minutes = totalMinutes % 60;
    return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  }

  async function loadItem(itemId: string) {
    // Reset state up front: this component instance is reused (not
    // remounted) when navigating between two /item/[id] routes -- e.g.
    // clicking a "More Like This" tile -- so stale data from the previous
    // item would otherwise stick around while the new one loads.
    item = null;
    imageUrl = "";
    backdropUrl = "";
    seasons = null;
    seasonImages = {};
    nextEpisodes = null;
    nextEpisodeImages = {};
    similarItems = null;
    nextUpEpisode = null;
    similarImages = {};
    error = "";
    loading = true;

    try {
      const loaded = await getItem(itemId);
      item = loaded;
      const backdropId = backdropSourceId(loaded);
      [imageUrl, backdropUrl] = await Promise.all([
        getImageUrl(itemId),
        backdropId ? getBackdropUrl(backdropId).catch(() => "") : Promise.resolve(""),
      ]);
      loading = false;

      if (loaded.Type === "Series") {
        getNextUp(itemId)
          .then((items) => (nextUpEpisode = items[0] ?? undefined))
          .catch(() => (nextUpEpisode = undefined));
        getSeasons(itemId)
          .then(async (s) => {
            seasons = s;
            // Seasons without their own artwork fall back to the series
            // poster: Jellyfin won't substitute one on its own -- asking
            // for a Primary image the season doesn't have just 404s.
            const entries = await Promise.all(
              s.map(async (season) => {
                const url = season.ImageTags?.Primary ? await getImageUrl(season.Id) : imageUrl;
                return [season.Id, url] as const;
              }),
            );
            seasonImages = Object.fromEntries(entries);
          })
          .catch(() => (seasons = []));
      } else if (loaded.Type === "Episode" && loaded.SeriesId && loaded.SeasonId) {
        getEpisodes(loaded.SeriesId, loaded.SeasonId)
          .then(async (seasonEpisodes) => {
            const next = seasonEpisodes.filter(
              (ep) => ep.Id !== loaded.Id && (ep.IndexNumber ?? 0) > (loaded.IndexNumber ?? 0),
            );
            nextEpisodes = next;
            const entries = await Promise.all(
              next.map(async (ep) => [ep.Id, await getImageUrl(ep.Id)] as const),
            );
            nextEpisodeImages = Object.fromEntries(entries);
          })
          .catch(() => (nextEpisodes = []));
      }

      if (loaded.Type === "Movie" || loaded.Type === "Series") {
        getSimilarItems(itemId)
          .then(async (items) => {
            similarItems = items;
            const entries = await Promise.all(
              items.map(async (si) => [si.Id, await getImageUrl(si.Id)] as const),
            );
            similarImages = Object.fromEntries(entries);
          })
          .catch(() => (similarItems = []));
      }
    } catch (e) {
      error = typeof e === "string" ? e : "Failed to load item";
      loading = false;
    }
  }

  $effect(() => {
    loadItem(page.params.id!);
  });
</script>

<Header />

<main>
  {#if loading}
    <section class="backdrop">
      <div class="skeleton backdrop-fill"></div>
    </section>
    <section class="detail">
      <div class="skeleton poster-skeleton"></div>
      <div class="info">
        <div class="skeleton skel-line skel-title"></div>
        <div class="skeleton skel-line skel-meta"></div>
        <div class="skeleton skel-line skel-overview"></div>
        <div class="skeleton skel-line skel-overview" style="width: 80%"></div>
      </div>
    </section>
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
          {#if item.OfficialRating}<span class="badge">{item.OfficialRating}</span>{/if}
          {#if item.CommunityRating}<span class="rating">&#9733; {item.CommunityRating.toFixed(1)}</span>{/if}
        </p>
        {#if item.Taglines && item.Taglines.length > 0}
          <p class="tagline">{item.Taglines[0]}</p>
        {/if}
        {#if item.Overview}
          <p class="overview">{item.Overview}</p>
        {/if}
        {#if item.Genres && item.Genres.length > 0}
          <div class="genres">
            {#each item.Genres as genre (genre)}
              <span class="genre">{genre}</span>
            {/each}
          </div>
        {/if}
        {#if item.Type === "Series"}
          <div class="actions">
            {#if nextUpEpisode === null}
              <div class="skeleton skel-btn"></div>
            {:else if nextUpEpisode}
              <a class="play" href={`/player/${nextUpEpisode.Id}`} title={nextUpEpisode.Name}>
                &#9658; {nextUpEpisode.UserData?.PlaybackPositionTicks ? "Resume" : (nextUpEpisode.IndexNumber === 1 && (nextUpEpisode.ParentIndexNumber ?? 1) === 1 ? "Play" : "Continue")}
                <span class="play-code">{episodeCode(nextUpEpisode)}</span>
              </a>
            {/if}
          <button class="play shuffle secondary" onclick={() => playRandomEpisode(item!.Id)} disabled={shuffling}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="16 3 21 3 21 8"></polyline>
              <line x1="4" y1="20" x2="21" y2="3"></line>
              <polyline points="21 16 21 21 16 21"></polyline>
              <line x1="15" y1="15" x2="21" y2="21"></line>
              <line x1="4" y1="4" x2="9" y2="9"></line>
            </svg>
            {shuffling ? "Picking…" : "Random Episode"}
          </button>
          </div>
          {#if shuffleError}<p class="error small">{shuffleError}</p>{/if}
        {:else}
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
        {#if seasons === null}
          <div class="skeleton-cards">
            {#each Array(4) as _}
              <div class="skeleton season-thumb"></div>
            {/each}
          </div>
        {:else if seasons.length === 0}
          <p class="dim">No seasons found.</p>
        {:else}
          <Carousel>
            {#each seasons as season (season.Id)}
              <a class="season-card" href={`/item/${item.Id}/season/${season.Id}`}>
                {#if seasonImages[season.Id]}
                  <img src={seasonImages[season.Id]} alt={season.Name} loading="lazy" />
                {:else}
                  <div class="skeleton season-thumb"></div>
                {/if}
                <span class="card-title">{season.Name}</span>
                {#if season.ProductionYear}<span class="card-subtitle">{season.ProductionYear}</span>{/if}
              </a>
            {/each}
          </Carousel>
        {/if}
      </section>
    {/if}

    {#if item.Type === "Episode"}
      <section class="next-up-section">
        <h2>Next Up</h2>
        {#if nextEpisodes === null}
          <div class="skeleton-cards">
            {#each Array(6) as _}
              <div class="skeleton next-up-thumb"></div>
            {/each}
          </div>
        {:else if nextEpisodes.length > 0}
          <Carousel>
            {#each nextEpisodes as ep (ep.Id)}
              <a class="next-up-card" href={`/player/${ep.Id}`} oncontextmenu={(e) => openItemMenu(e, ep)}>
                <div class="next-up-thumb">
                  {#if nextEpisodeImages[ep.Id]}
                    <img src={nextEpisodeImages[ep.Id]} alt={ep.Name} loading="lazy" />
                  {/if}
                  <WatchedOverlay item={ep} />
                </div>
                <span class="card-title">
                  {#if ep.IndexNumber}<span class="num">{ep.IndexNumber}.</span>{/if}
                  {ep.Name}
                </span>
                {#if ep.RunTimeTicks}<span class="card-subtitle">{formatRuntime(ep.RunTimeTicks)}</span>{/if}
              </a>
            {/each}
          </Carousel>
        {/if}
      </section>
    {/if}

    {#if item.Type === "Movie" || item.Type === "Series"}
      <section class="similar-section">
        <h2>More Like This</h2>
        {#if similarItems === null}
          <div class="skeleton-cards">
            {#each Array(6) as _}
              <div class="skeleton poster-thumb"></div>
            {/each}
          </div>
        {:else if similarItems.length > 0}
          <Carousel>
            {#each similarItems as si (si.Id)}
              <a class="poster-card" href={`/item/${si.Id}`} oncontextmenu={(e) => openItemMenu(e, si)}>
                <div class="poster-frame">
                  {#if similarImages[si.Id]}
                    <img src={similarImages[si.Id]} alt={si.Name} loading="lazy" />
                  {:else}
                    <div class="skeleton poster-thumb"></div>
                  {/if}
                  <WatchedOverlay item={si} />
                </div>
                <span class="card-title">{si.Name}</span>
              </a>
            {/each}
          </Carousel>
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
    height: 60vh;
    min-height: 440px;
    overflow: hidden;
  }

  .backdrop img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .backdrop-fill {
    position: absolute;
    inset: 0;
    border-radius: 0;
  }

  .skel-line {
    height: 1em;
  }

  .skel-title {
    width: 40%;
    height: 2.5rem;
  }

  .skel-meta {
    width: 30%;
  }

  .skel-overview {
    width: 60ch;
    max-width: 100%;
  }

  .backdrop-scrim {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 75%;
    /* Long, gradual fade down into the page background instead of the
       image being cropped short and cutting off abruptly -- most of the
       backdrop's top portion stays clear, and the bottom three-quarters
       eases into solid background color by the time it reaches the
       poster/info area below. */
    background: linear-gradient(180deg, transparent, var(--bg) 95%);
  }

  .detail {
    position: relative;
    margin-top: -160px;
    padding: 0 40px;
    display: flex;
    align-items: flex-start;
    gap: 2rem;
  }

  /* No fixed aspect-ratio/object-fit here on purpose: those would size the
     box to a guessed shape and then fit the image inside it, leaving the
     border/shadow tracing that guessed box rather than the actual image --
     exactly the "border doesn't wrap the poster" bug this replaces. Letting
     width drive height naturally means the element's own box always matches
     whatever the real image's aspect ratio is, so the border always wraps
     the image exactly. The loading placeholder below (before the image's
     real dimensions are known) is a separate element with its own guessed
     box, since there's nothing to size to yet.
  */
  .poster {
    display: block;
    width: 200px;
    height: auto;
    border-radius: 10px;
    border: 1px solid var(--border);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
    flex-shrink: 0;
  }

  .poster-skeleton {
    width: 200px;
    aspect-ratio: 2 / 3;
    border-radius: 10px;
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

  .next-up-section {
    padding: 2rem 40px 0;
  }

  .next-up-section h2 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }

  .next-up-card {
    flex-shrink: 0;
    width: 260px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    text-decoration: none;
    color: var(--text);
  }

  .next-up-thumb {
    position: relative;
    width: 260px;
    aspect-ratio: 16 / 9;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  .next-up-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .next-up-card:hover .next-up-thumb {
    outline: 2px solid var(--accent);
  }

  .next-up-card .num {
    color: var(--text-dim);
    font-weight: 400;
    margin-right: 0.25rem;
  }

  .card-subtitle {
    font-size: 0.8rem;
    color: var(--text-dim);
  }

  .skeleton-cards {
    display: flex;
    gap: 12px;
    overflow: hidden;
  }


  .meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    color: var(--text-dim);
    font-size: 0.9rem;
    margin: 0;
  }

  .meta .badge {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.1rem 0.45rem;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .meta .rating {
    color: var(--accent);
    font-weight: 700;
  }

  .tagline {
    margin: 0;
    font-style: italic;
    color: var(--text-dim);
    max-width: 60ch;
  }

  .overview {
    color: var(--text);
    line-height: 1.5;
    max-width: 60ch;
  }

  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .genre {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0.3rem 0.8rem;
    font-size: 0.8rem;
    color: var(--text-dim);
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

  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .actions .play {
    margin-top: 0;
  }

  .play-code {
    margin-left: 0.4rem;
    opacity: 0.6;
    font-weight: 600;
  }

  .skel-btn {
    width: 140px;
    height: 42px;
    border-radius: 8px;
  }

  .shuffle {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    border: none;
    font-size: 1rem;
    font-family: inherit;
  }

  .shuffle.secondary {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text);
    border: 1px solid rgba(255, 255, 255, 0.18);
  }

  .shuffle:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .error.small {
    margin: 0;
    font-size: 0.85rem;
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

  .season-card {
    flex-shrink: 0;
    width: 220px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    text-decoration: none;
    color: var(--text);
  }

  .season-card img,
  .season-thumb {
    width: 220px;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  .season-card:hover img {
    outline: 2px solid var(--accent);
  }

  .similar-section {
    padding: 2rem 40px 0;
  }

  .similar-section h2 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }

  .poster-card {
    flex-shrink: 0;
    width: 200px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    text-decoration: none;
    color: var(--text);
  }

  .poster-frame {
    position: relative;
    width: 200px;
    border-radius: 8px;
    overflow: hidden;
  }

  .poster-card img,
  .poster-thumb {
    display: block;
    width: 200px;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }

  .poster-card:hover img {
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
  }

  .error {
    color: var(--danger);
  }
</style>

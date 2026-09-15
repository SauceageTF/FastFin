<script lang="ts">
  let { children } = $props();
  let scrollEl: HTMLDivElement;

  // Each arrow is only shown when there's actually somewhere to scroll in
  // that direction -- a row that fits entirely on screen gets neither, and
  // a row scrolled to one end loses that end's arrow.
  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);

  function updateArrows() {
    if (!scrollEl) return;
    const { scrollLeft, scrollWidth, clientWidth } = scrollEl;
    canScrollLeft = scrollLeft > 1;
    canScrollRight = scrollLeft + clientWidth < scrollWidth - 1;
  }

  function scroll(direction: number) {
    scrollEl?.scrollBy({ left: direction * scrollEl.clientWidth * 0.9, behavior: "smooth" });
  }

  $effect(() => {
    updateArrows();
    // Content is usually still filling in (cards mount, posters load and
    // take their real size) after this first runs, so re-check whenever the
    // scroll container or anything inside it changes size.
    const observer = new ResizeObserver(updateArrows);
    observer.observe(scrollEl);
    for (const child of scrollEl.children) observer.observe(child);
    const mutations = new MutationObserver(() => {
      for (const child of scrollEl.children) observer.observe(child);
      updateArrows();
    });
    mutations.observe(scrollEl, { childList: true });
    return () => {
      observer.disconnect();
      mutations.disconnect();
    };
  });
</script>

<div class="carousel">
  {#if canScrollLeft}
    <button class="arrow left" onclick={() => scroll(-1)} aria-label="Scroll left">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6"></polyline>
      </svg>
    </button>
  {/if}
  <div class="row-scroll" bind:this={scrollEl} onscroll={updateArrows}>
    {@render children()}
  </div>
  {#if canScrollRight}
    <button class="arrow right" onclick={() => scroll(1)} aria-label="Scroll right">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 18 15 12 9 6"></polyline>
      </svg>
    </button>
  {/if}
</div>

<style>
  .carousel {
    position: relative;
  }

  .row-scroll {
    display: flex;
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 4px;
    scrollbar-width: none;
  }

  .row-scroll::-webkit-scrollbar {
    display: none;
  }

  .arrow {
    position: absolute;
    top: 0;
    bottom: 4px;
    width: 56px;
    display: flex;
    align-items: center;
    border: none;
    color: var(--text);
    cursor: pointer;
    z-index: 2;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .carousel:hover .arrow {
    opacity: 1;
  }

  .arrow.left {
    left: 0;
    justify-content: flex-start;
    padding-left: 4px;
    background: linear-gradient(90deg, var(--bg) 15%, transparent);
  }

  .arrow.right {
    right: 0;
    justify-content: flex-end;
    padding-right: 4px;
    background: linear-gradient(270deg, var(--bg) 15%, transparent);
  }
</style>

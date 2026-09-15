<script lang="ts">
  import "../app.css";
  import "$lib/theme";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import ItemContextMenu from "$lib/ItemContextMenu.svelte";
  import { itemMenu } from "$lib/contextMenu";
  let { children } = $props();

  // App-wide keyboard shortcuts for browsing: `/` or Ctrl+K jumps to
  // search, Escape goes back a page. Skipped on the player routes (this
  // layout also wraps the HUD overlay window and the main window's player
  // page, both of which have their own key handling) and while typing in a
  // field, where `/` and Escape mean something else.
  function handleKeydown(event: KeyboardEvent) {
    if (page.url.pathname.startsWith("/player/") || page.url.pathname === "/login") return;
    const target = event.target as HTMLElement | null;
    const typing =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target?.isContentEditable === true;

    if ((event.key === "k" && event.ctrlKey) || (event.key === "/" && !typing)) {
      event.preventDefault();
      if (page.url.pathname !== "/search") goto("/search");
      return;
    }
    // An open context menu claims Escape for itself (see ItemContextMenu).
    if (event.key === "Escape" && !typing && !$itemMenu && page.url.pathname !== "/library") {
      history.back();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{@render children()}

{#if !page.url.pathname.startsWith("/player/")}
  <ItemContextMenu />
{/if}

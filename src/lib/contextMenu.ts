import { writable } from "svelte/store";
import type { Item } from "./types";

// A single app-wide "item context menu" (right-click a poster/episode ->
// mark watched/unwatched). Rendered once in the root layout; any card opens
// it by calling `openItemMenu` from its contextmenu handler. The item passed
// in is mutated in place when the watched state changes -- every list in
// the app holds its items in a `$state` array, so that mutation is reactive
// and the card updates without the page needing its own callback.
export interface ItemMenuState {
  x: number;
  y: number;
  item: Item;
}

export const itemMenu = writable<ItemMenuState | null>(null);

export function openItemMenu(event: MouseEvent, item: Item) {
  event.preventDefault();
  event.stopPropagation();
  itemMenu.set({ x: event.clientX, y: event.clientY, item });
}

export function closeItemMenu() {
  itemMenu.set(null);
}

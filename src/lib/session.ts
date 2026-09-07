import { writable } from "svelte/store";

// null = not yet checked, false = logged out, true = logged in
export const isLoggedIn = writable<boolean | null>(null);

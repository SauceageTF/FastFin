<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { tryRestoreSession } from "$lib/jellyfinClient";
  import { isLoggedIn } from "$lib/session";

  onMount(async () => {
    const restored = await tryRestoreSession();
    isLoggedIn.set(restored);
    goto(restored ? "/library" : "/login", { replaceState: true });
  });
</script>

<main class="splash">
  <p>Loading FastFin&hellip;</p>
</main>

<style>
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: var(--text-dim);
  }
</style>

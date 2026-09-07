<script lang="ts">
  import { goto } from "$app/navigation";
  import { login } from "$lib/jellyfinClient";
  import { isLoggedIn } from "$lib/session";

  let serverUrl = $state("");
  let username = $state("");
  let password = $state("");
  let error = $state("");
  let loading = $state(false);

  async function handleSubmit(event: Event) {
    event.preventDefault();
    error = "";
    loading = true;
    try {
      await login(serverUrl, username, password);
      isLoggedIn.set(true);
      goto("/library");
    } catch (e) {
      error = typeof e === "string" ? e : "Login failed";
    } finally {
      loading = false;
    }
  }
</script>

<main class="login">
  <form class="card" onsubmit={handleSubmit}>
    <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="var(--text)" stroke-width="2.5" stroke-linecap="round">
      <line x1="4" y1="6" x2="20" y2="6"></line>
      <line x1="4" y1="12" x2="20" y2="12"></line>
      <line x1="4" y1="18" x2="20" y2="18"></line>
    </svg>
    <h1>Saucefin</h1>
    <p class="subtitle">Connect to your Jellyfin server</p>

    <label>
      Server URL
      <input type="url" placeholder="https://jellyfin.example.com" bind:value={serverUrl} required />
    </label>

    <label>
      Username
      <input type="text" bind:value={username} required />
    </label>

    <label>
      Password
      <input type="password" bind:value={password} />
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <button type="submit" disabled={loading}>
      {loading ? "Signing in…" : "Sign in"}
    </button>
  </form>
</main>

<style>
  .login {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
  }

  .card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 2.5rem;
    width: 340px;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  h1 {
    margin: 0;
    font-size: 1.6rem;
  }

  .subtitle {
    margin: 0 0 0.5rem;
    color: var(--text-dim);
    font-size: 0.9rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.85rem;
    color: var(--text-dim);
  }

  input {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.55rem 0.7rem;
    color: var(--text);
    font-size: 0.95rem;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  button {
    margin-top: 0.5rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 8px;
    padding: 0.65rem;
    font-size: 0.95rem;
    font-weight: 600;
  }

  button:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .error {
    color: var(--danger);
    font-size: 0.85rem;
    margin: 0;
  }
</style>

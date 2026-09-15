<script lang="ts">
  import { goto } from "$app/navigation";
  import { logout, getLibraries, getPlaylists } from "$lib/jellyfinClient";
  import { isLoggedIn } from "$lib/session";
  import { themeColor, type ThemeColor } from "$lib/theme";
  import {
    LANGUAGE_OPTIONS,
    getPreferredAudio,
    setPreferredAudio,
    getPreferredSubtitle,
    setPreferredSubtitle,
  } from "$lib/prefs";
  import type { Library, Item } from "$lib/types";

  const themeOptions: { value: ThemeColor; label: string; swatch: string }[] = [
    { value: "white", label: "White", swatch: "#f2f1f6" },
    { value: "teal", label: "Teal", swatch: "#2dd4c8" },
    { value: "ember", label: "Ember", swatch: "linear-gradient(135deg,#ff7a3d,#c62a0a)" },
  ];

  let sidebarOpen = $state(false);
  let settingsOpen = $state(false);
  let loadedSidebarData = false;
  let libraries = $state<Library[]>([]);
  let playlists = $state<Item[]>([]);
  let sidebarError = $state("");

  // Playback language preferences -- read by the player HUD when a new file's
  // tracks are known (see the HUD's applyLanguagePreferences).
  let prefAudio = $state(getPreferredAudio());
  let prefSubtitle = $state(getPreferredSubtitle());

  function changeAudio(event: Event) {
    prefAudio = (event.target as HTMLSelectElement).value;
    setPreferredAudio(prefAudio);
  }

  function changeSubtitle(event: Event) {
    prefSubtitle = (event.target as HTMLSelectElement).value;
    setPreferredSubtitle(prefSubtitle);
  }

  async function toggleSidebar() {
    settingsOpen = false;
    sidebarOpen = !sidebarOpen;
    if (sidebarOpen && !loadedSidebarData) {
      loadedSidebarData = true;
      try {
        const [libs, lists] = await Promise.all([getLibraries(), getPlaylists().catch(() => [])]);
        libraries = libs;
        playlists = lists;
      } catch (e) {
        sidebarError = typeof e === "string" ? e : "Failed to load sidebar";
      }
    }
  }

  function toggleSettings() {
    sidebarOpen = false;
    settingsOpen = !settingsOpen;
  }

  function closeOverlays() {
    sidebarOpen = false;
    settingsOpen = false;
  }

  async function handleLogout() {
    await logout();
    isLoggedIn.set(false);
    goto("/login");
  }
</script>

<header>
  <div class="left">
    <button class="logo-btn" onclick={toggleSidebar} aria-label="Toggle menu">
      <svg width="30" height="30" viewBox="0 0 24 24" fill="none" stroke="var(--text)" stroke-width="2.5" stroke-linecap="round">
        <line x1="4" y1="6" x2="20" y2="6"></line>
        <line x1="4" y1="12" x2="20" y2="12"></line>
        <line x1="4" y1="18" x2="20" y2="18"></line>
      </svg>
    </button>
    <a class="home-link" href="/library">Home</a>
  </div>
  <div class="right">
    <a class="icon-btn" href="/search" title="Search" aria-label="Search">
      <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="7"></circle>
        <line x1="21" y1="21" x2="16.2" y2="16.2"></line>
      </svg>
    </a>
    <button class="icon-btn" onclick={toggleSettings} title="Settings" aria-label="Toggle settings">
      <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"></path>
      </svg>
    </button>
    <button class="icon-btn" onclick={handleLogout} title="Sign out" aria-label="Sign out">
      <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
        <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
        <polyline points="16 17 21 12 16 7"></polyline>
        <line x1="21" y1="12" x2="9" y2="12"></line>
      </svg>
    </button>

    {#if settingsOpen}
      <div class="settings-popover">
        <div class="popover-label">Theme Color</div>
        <div class="swatches">
          {#each themeOptions as option (option.value)}
            <button
              class="swatch"
              class:selected={$themeColor === option.value}
              onclick={() => themeColor.set(option.value)}
              aria-label={option.label}
              title={option.label}
            >
              <span class="dot" style={`background:${option.swatch}`}></span>
            </button>
          {/each}
        </div>

        <div class="popover-label spaced">Preferred Audio</div>
        <select class="pref" value={prefAudio} onchange={changeAudio}>
          <option value="">Default</option>
          {#each LANGUAGE_OPTIONS as lang (lang.code)}
            <option value={lang.code}>{lang.label}</option>
          {/each}
        </select>

        <div class="popover-label spaced">Preferred Subtitles</div>
        <select class="pref" value={prefSubtitle} onchange={changeSubtitle}>
          <option value="">Default</option>
          <option value="off">Off</option>
          {#each LANGUAGE_OPTIONS as lang (lang.code)}
            <option value={lang.code}>{lang.label}</option>
          {/each}
        </select>

        <div class="shortcuts">
          <div class="popover-label spaced">Shortcuts</div>
          <div class="shortcut"><kbd>/</kbd><span>Search</span></div>
          <div class="shortcut"><kbd>Esc</kbd><span>Back</span></div>
          <div class="shortcut"><kbd>Space</kbd><span>Play / pause</span></div>
          <div class="shortcut"><kbd>&larr;</kbd><kbd>&rarr;</kbd><span>Seek 10s</span></div>
          <div class="shortcut"><kbd>&uarr;</kbd><kbd>&darr;</kbd><span>Volume</span></div>
          <div class="shortcut"><kbd>M</kbd><span>Mute</span></div>
          <div class="shortcut"><kbd>F</kbd><span>Fullscreen</span></div>
          <div class="shortcut"><kbd>S</kbd><span>Skip intro</span></div>
          <div class="shortcut"><kbd>N</kbd><span>Next episode</span></div>
        </div>
      </div>
    {/if}
  </div>
</header>

{#if sidebarOpen || settingsOpen}
  <div class="scrim" onclick={closeOverlays} role="presentation"></div>
{/if}

{#if sidebarOpen}
  <nav class="sidebar">
    {#if sidebarError}
      <p class="dim">{sidebarError}</p>
    {:else}
      <div class="section">
        <h3>Collections</h3>
        {#if libraries.length === 0}
          <p class="dim">No libraries found.</p>
        {:else}
          {#each libraries as lib (lib.Id)}
            <a class="entry" href={`/library/${lib.Id}`} onclick={closeOverlays}>{lib.Name}</a>
          {/each}
        {/if}
      </div>

      <div class="section">
        <h3>Playlists</h3>
        {#if playlists.length === 0}
          <p class="dim">No playlists found.</p>
        {:else}
          {#each playlists as playlist (playlist.Id)}
            <a class="entry" href={`/library/${playlist.Id}`} onclick={closeOverlays}>{playlist.Name}</a>
          {/each}
        {/if}
      </div>
    {/if}
  </nav>
{/if}

<style>
  header {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 6;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 40px;
    height: 68px;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.5), transparent);
  }

  .left {
    display: flex;
    align-items: center;
    gap: 22px;
  }

  .logo-btn {
    display: flex;
    background: transparent;
    border: none;
    padding: 4px;
    border-radius: 8px;
  }

  .logo-btn:hover {
    background: var(--bg-hover);
  }

  .home-link {
    color: var(--text);
    text-decoration: none;
    font-size: 0.875rem;
    font-weight: 600;
  }

  .right {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: 999px;
    color: var(--text-dim);
    background: transparent;
    border: none;
    text-decoration: none;
  }

  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .settings-popover {
    position: absolute;
    top: 48px;
    right: 44px;
    z-index: 11;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px 16px;
    box-shadow: 0 16px 32px rgba(0, 0, 0, 0.5);
  }

  .popover-label {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-dim);
    margin-bottom: 10px;
  }

  .swatches {
    display: flex;
    gap: 10px;
  }

  .popover-label.spaced {
    margin-top: 16px;
  }

  .pref {
    width: 100%;
    min-width: 200px;
    padding: 7px 10px;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    outline: none;
  }

  .pref:focus {
    border-color: var(--accent);
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .shortcut {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: var(--text-dim);
  }

  .shortcut span {
    margin-left: 4px;
  }

  kbd {
    display: inline-block;
    min-width: 22px;
    padding: 2px 6px;
    font-family: inherit;
    font-size: 0.7rem;
    font-weight: 700;
    text-align: center;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 5px;
  }

  .swatch {
    background: transparent;
    border: none;
    padding: 2px;
    border-radius: 999px;
  }

  .swatch.selected {
    box-shadow: 0 0 0 2px var(--bg-elevated), 0 0 0 3px var(--text);
  }

  .dot {
    display: block;
    width: 22px;
    height: 22px;
    border-radius: 999px;
  }

  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 10;
  }

  .sidebar {
    position: fixed;
    top: 0;
    bottom: 0;
    left: 0;
    width: 280px;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
    z-index: 11;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    overflow-y: auto;
    animation: slide-in 0.18s ease-out;
  }

  @keyframes slide-in {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(0);
    }
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .section h3 {
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-dim);
    margin: 0 0 8px;
  }

  .entry {
    display: block;
    padding: 10px 12px;
    border-radius: 8px;
    color: var(--text);
    text-decoration: none;
    font-size: 0.875rem;
    font-weight: 600;
  }

  .entry:hover {
    background: var(--bg-hover);
  }

  .dim {
    color: var(--text-dim);
    font-size: 0.8125rem;
  }
</style>

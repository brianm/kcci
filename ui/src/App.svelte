<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { getStats, type Stats } from './lib/api';
  import Search from './routes/Search.svelte';
  import Browse from './routes/Browse.svelte';
  import Import from './routes/Import.svelte';

  let currentRoute = 'search';
  let stats: Stats | null = null;
  let query = '';
  let searchInput: HTMLInputElement;

  onMount(async () => {
    try {
      stats = await getStats();
      if (stats.total_books === 0) {
        currentRoute = 'import';
      }
    } catch (e) {
      console.error('Failed to get stats:', e);
    }
    if (currentRoute === 'search') {
      await tick();
      searchInput?.focus();
    }
  });

  function navigate(route: string) {
    currentRoute = route;
    if (route === 'search') {
      tick().then(() => searchInput?.focus());
    }
  }

  function refreshStats() {
    getStats().then(s => stats = s);
  }
</script>

<div class="container">
  <header>
    {#if currentRoute === 'search'}
      <input
        type="text"
        class="header-search"
        bind:this={searchInput}
        bind:value={query}
        placeholder="Search your library..."
      />
    {/if}
    <nav>
      <a href="#/" class:active={currentRoute === 'search'} on:click|preventDefault={() => navigate('search')}>Search</a>
      <a href="#/books" class:active={currentRoute === 'browse'} on:click|preventDefault={() => navigate('browse')}>Browse</a>
      <a href="#/import" class:active={currentRoute === 'import'} on:click|preventDefault={() => navigate('import')}>Import</a>
    </nav>
  </header>

  <main>
    {#if currentRoute === 'search'}
      <Search {query} {searchInput} />
    {:else if currentRoute === 'browse'}
      <Browse {stats} />
    {:else if currentRoute === 'import'}
      <Import {stats} on:complete={refreshStats} />
    {/if}
  </main>
</div>

<style>
  :global(:root) {
    --bg: #fafafa;
    --bg-light: #ffffff;
    --accent: #2563eb;
    --accent-light: #dbeafe;
    --text: #1f2937;
    --text-dim: #6b7280;
    --border: #e5e7eb;
  }

  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(html, body) {
    height: auto;
    min-height: 100%;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg);
    color: var(--text);
    line-height: 1.6;
  }

  .container {
    max-width: 900px;
    margin: 0 auto;
    padding: 0 1rem;
  }

  header {
    display: flex;
    align-items: center;
    gap: 2rem;
    padding: 1rem 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 1rem;
  }

  .header-search {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-light);
    color: var(--text);
    font-size: 0.95rem;
  }

  .header-search:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-light);
  }

  nav {
    display: flex;
    gap: 2rem;
    margin-left: auto;
  }

  nav a {
    color: var(--text-dim);
    text-decoration: none;
    font-size: 1rem;
  }

  nav a:hover {
    color: var(--accent);
  }

  nav a.active {
    color: var(--text);
    font-weight: 500;
  }
</style>

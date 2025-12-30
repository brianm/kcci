<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats, type Stats } from './lib/api';
  import Search from './routes/Search.svelte';
  import Browse from './routes/Browse.svelte';
  import Import from './routes/Import.svelte';

  let currentRoute = 'search';
  let stats: Stats | null = null;

  onMount(async () => {
    try {
      stats = await getStats();
    } catch (e) {
      console.error('Failed to get stats:', e);
    }
  });

  function navigate(route: string) {
    currentRoute = route;
  }

  function refreshStats() {
    getStats().then(s => stats = s);
  }
</script>

<div class="container">
  <header>
    <h1><a href="#/" on:click|preventDefault={() => navigate('search')}>KCCI</a></h1>
    <nav>
      <a href="#/" class:active={currentRoute === 'search'} on:click|preventDefault={() => navigate('search')}>Search</a>
      <a href="#/books" class:active={currentRoute === 'browse'} on:click|preventDefault={() => navigate('browse')}>Browse</a>
      <a href="#/import" class:active={currentRoute === 'import'} on:click|preventDefault={() => navigate('import')}>Import</a>
    </nav>
  </header>

  <main>
    {#if currentRoute === 'search'}
      <Search />
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
    justify-content: space-between;
    align-items: center;
    padding: 1rem 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 1rem;
  }

  header h1 {
    font-size: 1.5rem;
    font-weight: 700;
  }

  header h1 a {
    color: var(--text);
    text-decoration: none;
  }

  nav {
    display: flex;
    gap: 2rem;
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

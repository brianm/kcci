<script lang="ts">
  import { search, type Book } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';

  let query = '';
  let mode: 'semantic' | 'fts' = 'semantic';
  let results: Book[] = [];
  let loading = false;
  let debounceTimer: ReturnType<typeof setTimeout>;

  async function doSearch() {
    if (!query.trim()) {
      results = [];
      return;
    }

    loading = true;
    try {
      results = await search(query, mode);
    } catch (e) {
      console.error('Search failed:', e);
      results = [];
    } finally {
      loading = false;
    }
  }

  function handleInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(doSearch, 300);
  }

  function handleModeChange() {
    if (query.trim()) {
      doSearch();
    }
  }
</script>

<div class="search-box">
  <input
    type="text"
    bind:value={query}
    on:input={handleInput}
    placeholder="Search your library..."
  />
  <select bind:value={mode} on:change={handleModeChange}>
    <option value="semantic">Semantic</option>
    <option value="fts">Keyword</option>
  </select>
</div>

{#if loading}
  <p class="status">Searching...</p>
{:else if results.length > 0}
  <p class="results-count">{results.length} results for "{query}"</p>
{/if}

{#each results as book (book.asin)}
  <BookCard {book} showScore={mode === 'semantic'} />
{/each}

<style>
  .search-box {
    display: flex;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .search-box input[type="text"] {
    flex: 1;
    padding: 0.75rem 1rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-light);
    color: var(--text);
    font-size: 1rem;
  }

  .search-box input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-light);
  }

  .search-box select {
    padding: 0.75rem 1rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-light);
    color: var(--text);
    font-size: 0.95rem;
    cursor: pointer;
  }

  .status, .results-count {
    color: var(--text-dim);
    margin-bottom: 1rem;
    font-size: 0.95rem;
  }
</style>

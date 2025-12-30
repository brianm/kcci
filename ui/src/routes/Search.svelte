<script lang="ts">
  import { tick, onMount } from 'svelte';
  import { search, type Book } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';

  let query = '';
  let mode: 'semantic' | 'fts' = 'semantic';
  let results: Book[] = [];
  let loading = false;
  let debounceTimer: ReturnType<typeof setTimeout>;
  let selectedIndex = -1;
  let expandedAsin: string | null = null;
  let resultsContainer: HTMLElement;
  let searchInput: HTMLInputElement;

  onMount(() => {
    searchInput?.focus();
  });

  async function doSearch() {
    if (!query.trim()) {
      results = [];
      selectedIndex = -1;
      expandedAsin = null;
      return;
    }

    loading = true;
    try {
      results = await search(query, mode);
      selectedIndex = results.length > 0 ? 0 : -1;
      expandedAsin = null;
    } catch (e) {
      console.error('Search failed:', e);
      results = [];
      selectedIndex = -1;
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

  async function handleKeydown(e: KeyboardEvent) {
    if (results.length === 0) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
      await scrollToSelected();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      await scrollToSelected();
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      e.preventDefault();
      const book = results[selectedIndex];
      expandedAsin = expandedAsin === book.asin ? null : book.asin;
    }
  }

  async function scrollToSelected() {
    await tick();
    if (resultsContainer && selectedIndex >= 0) {
      const cards = resultsContainer.querySelectorAll('.book-card');
      if (cards[selectedIndex]) {
        cards[selectedIndex].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    }
  }

  function handleCardClick(asin: string) {
    expandedAsin = expandedAsin === asin ? null : asin;
  }
</script>

<div class="search-box">
  <input
    type="text"
    bind:this={searchInput}
    bind:value={query}
    on:input={handleInput}
    on:keydown={handleKeydown}
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

<div class="results" bind:this={resultsContainer}>
  {#each results as book, index (book.asin)}
    <BookCard
      {book}
      showScore={mode === 'semantic'}
      selected={index === selectedIndex}
      expanded={expandedAsin === book.asin}
      on:click={() => handleCardClick(book.asin)}
    />
  {/each}
</div>

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

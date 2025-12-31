<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { search, getModelStatus, type Book } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';

  export let query = '';
  export let searchInput: HTMLInputElement | undefined = undefined;

  let results: Book[] = [];
  let loading = false;
  let debounceTimer: ReturnType<typeof setTimeout>;
  let selectedIndex = -1;
  let expandedAsin: string | null = null;
  let resultsContainer: HTMLElement;
  let modelAvailable = true; // Assume available, check on mount

  onMount(async () => {
    try {
      const status = await getModelStatus();
      modelAvailable = status.available;
    } catch (e) {
      console.error('Failed to check model status:', e);
      modelAvailable = false;
    }
  });

  $: {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(doSearch, 300);
    query; // dependency
  }

  async function doSearch() {
    if (!query.trim()) {
      results = [];
      selectedIndex = -1;
      expandedAsin = null;
      return;
    }

    loading = true;
    try {
      // Use semantic search if model available, otherwise fall back to FTS
      const mode = modelAvailable ? 'semantic' : 'fts';
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

  async function handleKeydown(e: KeyboardEvent) {
    // Only handle when search input is focused
    if (document.activeElement !== searchInput) return;
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

  function handleCardClick(index: number, asin: string) {
    selectedIndex = index;
    expandedAsin = expandedAsin === asin ? null : asin;
  }

  function handleCardMouseEnter(index: number) {
    selectedIndex = index;
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if !modelAvailable}
  <p class="model-notice">Using keyword search. <a href="#import">Download the model</a> to enable semantic search.</p>
{/if}

{#if loading}
  <p class="status">Searching...</p>
{:else if results.length > 0}
  <p class="results-count">{results.length} results for "{query}"</p>
{/if}

<div class="results" bind:this={resultsContainer}>
  {#each results as book, index (book.asin)}
    <BookCard
      {book}
      showScore={true}
      selected={index === selectedIndex}
      expanded={expandedAsin === book.asin}
      on:click={() => handleCardClick(index, book.asin)}
      on:mouseenter={() => handleCardMouseEnter(index)}
    />
  {/each}
</div>

<style>
  .model-notice {
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    font-size: 0.9rem;
    color: var(--text-dim);
  }

  .model-notice a {
    color: var(--accent);
  }

  .status, .results-count {
    color: var(--text-dim);
    margin-bottom: 1rem;
    font-size: 0.95rem;
  }
</style>

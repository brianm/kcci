<script lang="ts">
  import { onMount } from 'svelte';
  import { browseFiltered, type Book, type Stats, type SearchFilter } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';

  interface Props {
    stats?: Stats | null;
    filters?: SearchFilter[];
  }

  let { stats = null, filters = $bindable([]) }: Props = $props();

  let books: Book[] = $state([]);
  let currentPage = $state(0);
  let totalPages = $state(1);
  let loading = $state(false);
  let error: string | null = $state(null);
  let selectedIndex = $state(-1);
  let expandedAsin: string | null = $state(null);
  let sentinel: HTMLElement | undefined = $state();

  // Sorting state
  let sortBy: 'title' | 'author' | 'year' = $state('title');
  let sortDir: 'asc' | 'desc' = $state('asc');
  let lastFilterJson = $state('');

  onMount(async () => {
    loadMore();

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !loading && currentPage < totalPages) {
          loadMore();
        }
      },
      { rootMargin: '200px' }
    );

    if (sentinel) {
      observer.observe(sentinel);
    }
    return () => observer.disconnect();
  });

  async function loadMore() {
    if (loading || currentPage >= totalPages) return;

    loading = true;
    error = null;
    try {
      const data = await browseFiltered({
        filters,
        page: currentPage + 1,
        perPage: 50,
        sortBy,
        sortDir,
      });
      books = [...books, ...data.books];
      currentPage = data.page;
      totalPages = data.total_pages;
    } catch (e) {
      console.error('Browse: loadMore error:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function resetAndReload() {
    books = [];
    currentPage = 0;
    totalPages = 1;
    loadMore();
  }

  // Watch for filter changes from parent
  $effect(() => {
    const filterJson = JSON.stringify(filters);
    if (filterJson !== lastFilterJson) {
      lastFilterJson = filterJson;
      resetAndReload();
    }
  });

  function handleSortChange() {
    resetAndReload();
  }

  function toggleSortDir() {
    sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    resetAndReload();
  }

  function handleCardClick(index: number, asin: string) {
    selectedIndex = index;
    expandedAsin = expandedAsin === asin ? null : asin;
  }

  function handleCardMouseEnter(index: number) {
    selectedIndex = index;
  }
</script>

<div class="browse-page">
  <div class="controls">
    <div class="sort-controls">
      <label>
        Sort by:
        <select bind:value={sortBy} onchange={handleSortChange}>
          <option value="title">Title</option>
          <option value="author">Author</option>
          <option value="year">Year</option>
        </select>
      </label>
      <button class="sort-dir" onclick={toggleSortDir} title="Toggle sort direction">
        {sortDir === 'asc' ? '↑' : '↓'}
      </button>
    </div>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="books">
    {#each books as book, index (book.asin)}
      <BookCard
        {book}
        selected={index === selectedIndex}
        expanded={expandedAsin === book.asin}
        onclick={() => handleCardClick(index, book.asin)}
        onmouseenter={() => handleCardMouseEnter(index)}
      />
    {/each}
  </div>

  <div bind:this={sentinel} class="sentinel">
    {#if loading}
      <p class="loading">Loading...</p>
    {:else if currentPage >= totalPages && books.length > 0}
      <p class="end">End of library ({books.length} books)</p>
    {:else if !loading && books.length === 0 && filters.length > 0}
      <p class="no-results">No books match your filters</p>
    {/if}
  </div>
</div>

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .sort-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .sort-controls label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    color: var(--text-dim);
  }

  .sort-controls select {
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .sort-controls select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .sort-dir {
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .sort-dir:hover {
    background: var(--border);
  }

  .loading, .error, .no-results {
    text-align: center;
    padding: 3rem;
    color: var(--text-dim);
  }

  .error {
    color: #dc2626;
  }

  .sentinel {
    text-align: center;
    padding: 2rem;
  }

  .end {
    color: var(--text-dim);
    font-size: 0.9rem;
  }
</style>

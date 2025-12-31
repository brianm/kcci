<script lang="ts">
  import { onMount } from 'svelte';
  import { listBooks, getSubjects, type Book, type PaginatedBooks, type Stats, type ListBooksOptions, type Filter } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';
  import FilterBuilder from '../components/FilterBuilder.svelte';

  export let stats: Stats | null = null;

  let books: Book[] = [];
  let currentPage = 0;
  let totalPages = 1;
  let loading = false;
  let error: string | null = null;
  let selectedIndex = -1;
  let expandedAsin: string | null = null;
  let sentinel: HTMLElement;

  // Sorting and filtering state
  let sortBy: 'title' | 'author' | 'year' = 'title';
  let sortDir: 'asc' | 'desc' = 'asc';
  let filters: Filter[] = [];
  let subjects: string[] = [];

  onMount(async () => {
    // Load subjects for filter dropdown
    try {
      subjects = await getSubjects();
    } catch (e) {
      console.error('Failed to load subjects:', e);
    }

    loadMore();

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !loading && currentPage < totalPages) {
          loadMore();
        }
      },
      { rootMargin: '200px' }
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  });

  async function loadMore() {
    if (loading || currentPage >= totalPages) return;

    loading = true;
    error = null;
    try {
      const options: ListBooksOptions = {
        page: currentPage + 1,
        perPage: 50,
        sortBy,
        sortDir,
      };
      if (filters.length > 0) {
        options.filters = filters;
      }
      const data = await listBooks(options);
      books = [...books, ...data.books];
      currentPage = data.page;
      totalPages = data.total_pages;
    } catch (e) {
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

  function handleSortChange() {
    resetAndReload();
  }

  function handleFiltersChange(event: CustomEvent<Filter[]>) {
    filters = event.detail;
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
        <select bind:value={sortBy} on:change={handleSortChange}>
          <option value="title">Title</option>
          <option value="author">Author</option>
          <option value="year">Year</option>
        </select>
      </label>
      <button class="sort-dir" on:click={toggleSortDir} title="Toggle sort direction">
        {sortDir === 'asc' ? '↑' : '↓'}
      </button>
    </div>
  </div>

  <div class="filter-section">
    <FilterBuilder {subjects} on:change={handleFiltersChange} />
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
        on:click={() => handleCardClick(index, book.asin)}
        on:mouseenter={() => handleCardMouseEnter(index)}
      />
    {/each}
  </div>

  <div bind:this={sentinel} class="sentinel">
    {#if loading}
      <p class="loading">Loading...</p>
    {:else if currentPage >= totalPages && books.length > 0}
      <p class="end">End of library</p>
    {/if}
  </div>
</div>

<style>
  .controls {
    display: flex;
    gap: 2rem;
    margin-bottom: 1rem;
    padding: 1rem 1.25rem;
    background: var(--bg-light);
    border-radius: 8px;
    border: 1px solid var(--border);
    flex-wrap: wrap;
    align-items: center;
  }

  .sort-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .controls label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    color: var(--text-dim);
  }

  .controls select {
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .controls select:focus {
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

  .filter-section {
    margin-bottom: 1.5rem;
    padding: 1rem 1.25rem;
    background: var(--bg-light);
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  .loading, .error {
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

<script lang="ts">
  import { onMount } from 'svelte';
  import { listBooks, type Book, type PaginatedBooks, type Stats } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';

  export let stats: Stats | null = null;

  let books: Book[] = [];
  let currentPage = 0;
  let totalPages = 1;
  let loading = false;
  let error: string | null = null;
  let selectedIndex = -1;
  let expandedAsin: string | null = null;
  let sentinel: HTMLElement;

  onMount(() => {
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
      const data = await listBooks(currentPage + 1, 50);
      books = [...books, ...data.books];
      currentPage = data.page;
      totalPages = data.total_pages;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
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

<div class="browse-page">
  {#if stats}
    <div class="stats">
      <div class="stat">
        <div class="stat-value">{stats.total_books}</div>
        <div class="stat-label">Books</div>
      </div>
      <div class="stat">
        <div class="stat-value">{stats.enriched}</div>
        <div class="stat-label">Enriched</div>
      </div>
      <div class="stat">
        <div class="stat-value">{stats.with_embeddings}</div>
        <div class="stat-label">Embedded</div>
      </div>
    </div>
  {/if}

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
  .stats {
    display: flex;
    gap: 2rem;
    margin-bottom: 2rem;
    padding: 1.25rem 1.5rem;
    background: var(--bg-light);
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  .stat {
    text-align: center;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: bold;
    color: var(--accent);
  }

  .stat-label {
    font-size: 0.85rem;
    color: var(--text-dim);
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

<script lang="ts">
  import { onMount } from 'svelte';
  import { listBooks, type Book, type PaginatedBooks, type Stats } from '../lib/api';
  import BookCard from '../components/BookCard.svelte';

  export let stats: Stats | null = null;

  let data: PaginatedBooks | null = null;
  let loading = false;
  let error: string | null = null;

  onMount(() => {
    loadPage(1);
  });

  async function loadPage(page: number) {
    loading = true;
    error = null;
    try {
      data = await listBooks(page, 50);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
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

  {#if loading && !data}
    <p class="loading">Loading...</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if data}
    <div class="books">
      {#each data.books as book (book.asin)}
        <BookCard {book} />
      {/each}
    </div>

    {#if data.total_pages > 1}
      <div class="pagination">
        {#if data.page > 1}
          <a href="javascript:void(0)" on:click={() => loadPage(data!.page - 1)}>Previous</a>
        {/if}
        <span>Page {data.page} of {data.total_pages}</span>
        {#if data.page < data.total_pages}
          <a href="javascript:void(0)" on:click={() => loadPage(data!.page + 1)}>Next</a>
        {/if}
      </div>
    {/if}
  {/if}
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

  .pagination {
    display: flex;
    gap: 1rem;
    justify-content: center;
    margin-top: 2rem;
    align-items: center;
  }

  .pagination a {
    padding: 0.6rem 1.25rem;
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    text-decoration: none;
    transition: border-color 0.15s;
  }

  .pagination a:hover {
    border-color: var(--accent);
  }

  .pagination span {
    color: var(--text-dim);
    padding: 0 1rem;
  }
</style>

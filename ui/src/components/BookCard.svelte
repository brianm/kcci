<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-shell';
  import { marked } from 'marked';
  import type { Book } from '../lib/api';

  export let book: Book;
  export let showScore = false;
  export let selected = false;
  export let expanded = false;

  const dispatch = createEventDispatcher();

  async function openExternal(url: string, event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    await open(url);
  }

  async function handleDescriptionClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    const anchor = target.closest('a');
    if (anchor?.href) {
      event.preventDefault();
      event.stopPropagation();
      await open(anchor.href);
    }
  }

  function getScore(): number {
    if (book.distance !== null) {
      return Math.round((2 - book.distance) * 50);
    }
    return 0;
  }

  function formatAuthors(): string {
    return book.authors.join('; ');
  }

  function renderDescription(): string {
    if (!book.description) return '';
    return marked.parse(book.description, { async: false }) as string;
  }

  function handleClick() {
    dispatch('click');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      dispatch('click');
    }
  }
</script>

<div
  class="book-card"
  class:expanded
  class:selected
  on:click={handleClick}
  on:keydown={handleKeydown}
  on:mouseenter={() => dispatch('mouseenter')}
  role="button"
  tabindex="-1"
>
  <div class="book-header">
    <div class="book-title">
      {#if showScore && book.distance !== null}
        <span class="book-score">{getScore()}%</span>
      {/if}
      {book.title}
    </div>
    <div class="book-author">{formatAuthors()}</div>
  </div>

  {#if expanded}
    <div class="book-details">
      {#if book.description}
        <div class="book-description" on:click={handleDescriptionClick}>{@html renderDescription()}</div>
      {/if}

      {#if book.subjects.length > 0}
        <div class="book-subjects">
          {#each book.subjects.slice(0, 10) as subject}
            <span class="tag">{subject}</span>
          {/each}
        </div>
      {/if}

      <div class="book-meta">
        {#if book.publish_year}
          <span>Published {book.publish_year}</span>
        {/if}
        <span>ASIN: {book.asin}</span>
        {#if book.openlibrary_key}
          <a
            href="https://openlibrary.org{book.openlibrary_key}"
            on:click={(e) => openExternal(`https://openlibrary.org${book.openlibrary_key}`, e)}
          >
            OpenLibrary
          </a>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .book-card {
    background: var(--bg-light);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 0.75rem;
    border: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
  }

  .book-card.selected {
    border-color: var(--accent);
    background: var(--accent-light);
  }

  .book-card.expanded {
    border-color: var(--accent);
    cursor: default;
  }

  .book-card.expanded .book-header {
    cursor: pointer;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.75rem;
  }

  .book-card.expanded .book-header:hover {
    color: var(--accent);
  }

  .book-title {
    font-weight: 600;
    margin-bottom: 0.25rem;
    color: var(--text);
  }

  .book-author {
    color: var(--text-dim);
    font-size: 0.9rem;
  }

  .book-score {
    display: inline-block;
    background: var(--accent-light);
    color: var(--accent);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
    margin-right: 0.5rem;
  }

  .book-description {
    font-size: 0.9rem;
    line-height: 1.6;
    color: var(--text);
    margin-bottom: 0.75rem;
  }

  .book-description :global(p) {
    margin-bottom: 0.5rem;
  }

  .book-description :global(a) {
    color: var(--accent);
  }

  .book-subjects {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
  }

  .tag {
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 0.25rem 0.75rem;
    border-radius: 20px;
    font-size: 0.8rem;
    color: var(--text-dim);
  }

  .book-meta {
    font-size: 0.8rem;
    color: var(--text-dim);
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }

  .book-meta span {
    margin-right: 0.5rem;
  }

  .book-meta span::after {
    content: " · ";
  }

  .book-meta span:last-of-type::after {
    content: "";
  }

  .book-meta a {
    color: var(--accent);
    text-decoration: none;
  }

  .book-meta a:hover {
    text-decoration: underline;
  }
</style>

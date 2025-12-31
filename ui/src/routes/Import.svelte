<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { syncLibrary, clearMetadata, type SyncProgress, type SyncStats, type Stats } from '../lib/api';

  const dispatch = createEventDispatcher();

  export let stats: Stats | null = null;

  let syncing = false;
  let progress: SyncProgress | null = null;
  let syncStats: SyncStats | null = null;
  let error: string | null = null;

  async function selectFile() {
    const path = await open({
      filters: [{ name: 'Webarchive', extensions: ['webarchive'] }]
    });

    if (path) {
      await startSync(path as string);
    }
  }

  async function startSync(path: string) {
    syncing = true;
    progress = null;
    syncStats = null;
    error = null;

    try {
      syncStats = await syncLibrary(path, (p) => {
        progress = p;
      });
      dispatch('complete');
    } catch (e) {
      error = String(e);
    } finally {
      syncing = false;
    }
  }

  function getProgressPercent(): number {
    if (!progress?.current || !progress?.total) return 0;
    return Math.round((progress.current / progress.total) * 100);
  }

  function getStageName(stage: string): string {
    const names: Record<string, string> = {
      'import': 'Importing',
      'enrich': 'Enriching',
      'embed': 'Embedding'
    };
    return names[stage] || stage;
  }

  async function reEnrich() {
    syncing = true;
    progress = null;
    syncStats = null;
    error = null;

    try {
      // Clear all metadata first
      await clearMetadata();

      // Then sync without a webarchive file (just enriches existing books)
      syncStats = await syncLibrary(undefined, (p) => {
        progress = p;
      });
      dispatch('complete');
    } catch (e) {
      error = String(e);
    } finally {
      syncing = false;
    }
  }
</script>

<div class="import-page">
  {#if stats}
    <div class="stats-info">
      Currently tracking <strong>{stats.total_books}</strong> books,
      <strong>{stats.enriched}</strong> enriched,
      <strong>{stats.with_embeddings}</strong> with embeddings.
    </div>
  {/if}

  <div class="instructions">
    <h3>How to get your Kindle library webarchive:</h3>
    <ol>
      <li>Open Safari and go to <a href="https://read.amazon.com" target="_blank" rel="noopener">read.amazon.com</a></li>
      <li>Sign in with your Amazon account</li>
      <li><strong>Scroll down repeatedly</strong> until all your books are loaded (the page lazy-loads as you scroll)</li>
      <li>From the menu bar, choose <strong>File &gt; Save As...</strong></li>
      <li>Set Format to <strong>"Web Archive"</strong></li>
      <li>Save the file, then click below to select it</li>
    </ol>
  </div>

  <button class="import-zone" on:click={selectFile} disabled={syncing}>
    <div class="import-zone-icon">📁</div>
    <div class="import-zone-text">
      {#if syncing}
        Syncing...
      {:else}
        Click to select a webarchive file
      {/if}
    </div>
  </button>

  {#if stats && stats.total_books > 0}
    <div class="reenrich-section">
      <button class="reenrich-btn" on:click={reEnrich} disabled={syncing}>
        Re-fetch metadata from OpenLibrary
      </button>
      <p class="reenrich-hint">
        Clears existing metadata and re-fetches from OpenLibrary for all {stats.total_books} books.
      </p>
    </div>
  {/if}

  {#if syncing && progress}
    <div class="sync-progress">
      <h3>{getStageName(progress.stage)}</h3>
      <div class="progress-bar">
        <div class="progress-bar-fill" style="width: {getProgressPercent()}%"></div>
      </div>
      <p class="progress-message">{progress.message}</p>
    </div>
  {/if}

  {#if syncStats}
    <div class="sync-complete">
      <div class="drop-icon">✓</div>
      <h3>Sync Complete</h3>
      <div class="sync-stats">
        <div>Imported: <strong>{syncStats.imported}</strong> new books</div>
        <div>Enriched: <strong>{syncStats.enriched}</strong> books</div>
        <div>Embedded: <strong>{syncStats.embedded}</strong> books</div>
      </div>
    </div>
  {/if}

  {#if error}
    <div class="sync-error">
      <div class="drop-icon">⚠️</div>
      <h3>Sync Failed</h3>
      <p>{error}</p>
    </div>
  {/if}
</div>

<style>
  .stats-info {
    margin-bottom: 1.5rem;
    color: var(--text-dim);
    font-size: 0.95rem;
  }

  .instructions {
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.5rem 2rem;
    margin-bottom: 1.5rem;
  }

  .instructions h3 {
    margin-bottom: 1rem;
    font-size: 1rem;
    color: var(--text);
  }

  .instructions ol {
    margin-left: 1.5rem;
    color: var(--text);
  }

  .instructions li {
    margin-bottom: 0.6rem;
    line-height: 1.6;
  }

  .instructions a {
    color: var(--accent);
  }

  .import-zone {
    display: block;
    width: 100%;
    border: 2px dashed var(--border);
    border-radius: 8px;
    padding: 2.5rem;
    text-align: center;
    margin-bottom: 1.5rem;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
    background: transparent;
    font-family: inherit;
  }

  .import-zone:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-light);
  }

  .import-zone:disabled {
    cursor: not-allowed;
    opacity: 0.7;
  }

  .import-zone-icon {
    font-size: 2.5rem;
    margin-bottom: 0.75rem;
  }

  .import-zone-text {
    color: var(--text-dim);
    font-size: 1rem;
  }

  .sync-progress {
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 2rem;
    text-align: center;
  }

  .sync-progress h3 {
    margin-bottom: 1rem;
    color: var(--text);
  }

  .progress-bar {
    width: 100%;
    height: 8px;
    background: var(--border);
    border-radius: 4px;
    overflow: hidden;
    margin: 1rem 0;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }

  .progress-message {
    color: var(--text-dim);
    font-size: 0.9rem;
    margin-top: 0.75rem;
  }

  .sync-complete {
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 2rem;
    text-align: center;
  }

  .sync-complete h3 {
    color: #16a34a;
    margin-bottom: 1rem;
  }

  .sync-complete .drop-icon {
    font-size: 2.5rem;
    margin-bottom: 0.75rem;
  }

  .sync-stats {
    text-align: left;
    max-width: 220px;
    margin: 0 auto;
  }

  .sync-stats div {
    padding: 0.35rem 0;
    color: var(--text);
  }

  .sync-error {
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 8px;
    padding: 2rem;
    text-align: center;
  }

  .sync-error h3 {
    color: #dc2626;
    margin-bottom: 0.75rem;
  }

  .sync-error .drop-icon {
    font-size: 2.5rem;
    margin-bottom: 0.75rem;
  }

  .sync-error p {
    color: #dc2626;
  }

  .reenrich-section {
    margin-top: 1.5rem;
    text-align: center;
  }

  .reenrich-btn {
    padding: 0.6rem 1.2rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-light);
    color: var(--text);
    font-size: 0.9rem;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
  }

  .reenrich-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-light);
  }

  .reenrich-btn:disabled {
    cursor: not-allowed;
    opacity: 0.7;
  }

  .reenrich-hint {
    margin-top: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
</style>

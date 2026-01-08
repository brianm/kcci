<script lang="ts">
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { Command } from '@tauri-apps/plugin-shell';
  import { syncLibrary, clearMetadata, exportCsv, type SyncProgress, type SyncStats, type Stats } from '../lib/api';

  async function openInSafari(url: string) {
    await Command.create('open-safari', ['-a', 'Safari', url]).execute();
  }

  interface Props {
    stats?: Stats | null;
    oncomplete?: () => void;
  }

  let { stats = null, oncomplete }: Props = $props();

  let syncing = $state(false);
  let progress: SyncProgress | null = $state(null);
  let syncStats: SyncStats | null = $state(null);
  let error: string | null = $state(null);

  let includeEnrichment = $state(false);
  let exporting = $state(false);

  async function selectFile() {
    const path = await open({
      filters: [{ name: 'Webarchive', extensions: ['webarchive', 'mhtml', 'html'] }]
    });

    if (path) {
      await startSync(path as string);
    }
  }

  async function selectAmazonFolder() {
    const path = await open({
      directory: true,
      title: 'Select Amazon Kindle Export Folder'
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
      oncomplete?.();
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

  async function continueEnrichment() {
    syncing = true;
    progress = null;
    syncStats = null;
    error = null;

    try {
      // Sync without a webarchive file - enriches books missing metadata, embeds books missing embeddings
      syncStats = await syncLibrary(undefined, (p) => {
        progress = p;
      });
      oncomplete?.();
    } catch (e) {
      error = String(e);
    } finally {
      syncing = false;
    }
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
      oncomplete?.();
    } catch (e) {
      error = String(e);
    } finally {
      syncing = false;
    }
  }

  async function exportLibrary() {
    const path = await save({
      filters: [{ name: 'CSV', extensions: ['csv'] }],
      defaultPath: 'ook-library.csv'
    });

    if (!path) return;

    exporting = true;
    error = null;

    try {
      await exportCsv(path, includeEnrichment);
    } catch (e) {
      error = String(e);
    } finally {
      exporting = false;
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

  <div class="import-methods">
    <div class="import-method">
      <div class="instructions">
        <h3>Option 1: Amazon Data Export (Recommended)</h3>
        <ol>
          <li>Go to <button class="link-button" onclick={() => openInSafari('https://www.amazon.com/hz/privacy-central/data-requests/preview.html')}>Amazon Download Your Data</button></li>
          <li>Check <strong>"Kindle"</strong> and click "Submit Request"</li>
          <li>Wait for email from Amazon (can take a few days)</li>
          <li>Download and extract the ZIP file</li>
          <li>Select the extracted <strong>"Kindle"</strong> folder below</li>
        </ol>
        <p class="method-note">This method provides purchase dates, ownership status, and more complete metadata.</p>
      </div>

      <button class="import-zone" onclick={selectAmazonFolder} disabled={syncing}>
        <div class="import-zone-icon">📂</div>
        <div class="import-zone-text">
          {#if syncing}
            Syncing...
          {:else}
            Click to select Amazon export folder
          {/if}
        </div>
      </button>
    </div>

    <div class="import-method">
      <div class="instructions">
        <h3>Option 2: Safari Webarchive</h3>
        <ol>
          <li>Open Safari and go to <button class="link-button" onclick={() => openInSafari('https://read.amazon.com')}>read.amazon.com</button></li>
          <li>Sign in with your Amazon account</li>
          <li><strong>Scroll down repeatedly</strong> until all your books are loaded</li>
          <li>From the menu bar, choose <strong>File &gt; Save As...</strong></li>
          <li>Set Format to <strong>"Web Archive"</strong> and save</li>
        </ol>
        <p class="method-note">Quick method if you don't want to wait for Amazon's data export.</p>
      </div>

      <button class="import-zone" onclick={selectFile} disabled={syncing}>
        <div class="import-zone-icon">📁</div>
        <div class="import-zone-text">
          {#if syncing}
            Syncing...
          {:else}
            Click to select a webarchive file
          {/if}
        </div>
      </button>
    </div>
  </div>

  {#if stats && stats.total_books > 0}
    <div class="reenrich-section">
      {#if stats.total_books > stats.enriched || stats.enriched > stats.with_embeddings}
        <button class="reenrich-btn primary" onclick={continueEnrichment} disabled={syncing}>
          Continue enrichment
        </button>
        <p class="reenrich-hint">
          Process {stats.total_books - stats.enriched} books missing metadata and {stats.enriched - stats.with_embeddings} missing embeddings.
        </p>
      {/if}
      <button class="reenrich-btn" onclick={reEnrich} disabled={syncing}>
        Re-fetch all metadata
      </button>
      <p class="reenrich-hint">
        Clears existing metadata and re-fetches from OpenLibrary for all {stats.total_books} books.
      </p>
    </div>

    <div class="export-section">
      <div class="export-controls">
        <button class="export-btn" onclick={exportLibrary} disabled={exporting || syncing}>
          {exporting ? 'Exporting...' : 'Export to CSV'}
        </button>
        <label class="export-checkbox">
          <input type="checkbox" bind:checked={includeEnrichment} disabled={exporting} />
          Include enrichment data
        </label>
      </div>
      <p class="export-hint">
        Export your library to a CSV file. Enrichment data includes descriptions, subjects, and OpenLibrary metadata.
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

  .import-methods {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }

  @media (max-width: 900px) {
    .import-methods {
      grid-template-columns: 1fr;
    }
  }

  .import-method {
    display: flex;
    flex-direction: column;
  }

  .instructions {
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.5rem 2rem;
    margin-bottom: 1rem;
    flex: 1;
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

  .method-note {
    margin-top: 1rem;
    font-size: 0.85rem;
    color: var(--text-dim);
    font-style: italic;
  }

  .link-button {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font: inherit;
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
  }

  .link-button:hover {
    color: var(--accent-hover, var(--accent));
  }

  .import-zone {
    display: block;
    width: 100%;
    border: 2px dashed var(--border);
    border-radius: 8px;
    padding: 2rem;
    text-align: center;
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

  .reenrich-btn.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .reenrich-btn.primary:hover:not(:disabled) {
    background: var(--accent-hover, var(--accent));
    border-color: var(--accent-hover, var(--accent));
  }

  .reenrich-hint {
    margin-top: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-dim);
  }

  .export-section {
    margin-top: 1.5rem;
    text-align: center;
  }

  .export-controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1rem;
  }

  .export-btn {
    padding: 0.6rem 1.2rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-light);
    color: var(--text);
    font-size: 0.9rem;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
  }

  .export-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-light);
  }

  .export-btn:disabled {
    cursor: not-allowed;
    opacity: 0.7;
  }

  .export-checkbox {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.9rem;
    color: var(--text);
    cursor: pointer;
  }

  .export-checkbox input {
    cursor: pointer;
  }

  .export-hint {
    margin-top: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
</style>

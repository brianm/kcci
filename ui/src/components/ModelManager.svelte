<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { getModelStatus, downloadModel, type ModelStatus, type DownloadProgress } from '../lib/api';

  const dispatch = createEventDispatcher();

  let status: ModelStatus | null = null;
  let downloading = false;
  let progress: DownloadProgress | null = null;
  let error: string | null = null;

  onMount(async () => {
    await checkStatus();
  });

  async function checkStatus() {
    try {
      status = await getModelStatus();
    } catch (e) {
      console.error('Failed to get model status:', e);
    }
  }

  async function startDownload() {
    downloading = true;
    progress = null;
    error = null;

    try {
      await downloadModel((p) => {
        progress = p;
      });
      await checkStatus();
      dispatch('downloaded');
    } catch (e) {
      error = String(e);
    } finally {
      downloading = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024 * 1024) {
      return `${(bytes / 1024).toFixed(1)} KB`;
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="model-manager">
  <h3>Semantic Search Model</h3>

  {#if status === null}
    <p class="status-loading">Checking model status...</p>
  {:else if status.available}
    <div class="status-available">
      <span class="status-icon">&#10003;</span>
      <span>Model downloaded</span>
    </div>
    <p class="hint">Semantic search is enabled.</p>
  {:else if downloading && progress}
    <div class="download-progress">
      <div class="progress-bar">
        <div class="progress-bar-fill" style="width: {progress.percent}%"></div>
      </div>
      <p class="progress-text">
        Downloading... {formatBytes(progress.bytes_downloaded)} / {formatBytes(progress.total_bytes)}
        ({progress.percent.toFixed(0)}%)
      </p>
    </div>
  {:else if downloading}
    <p class="status-loading">Starting download...</p>
  {:else}
    <div class="status-missing">
      <span class="status-icon">&#9675;</span>
      <span>Not downloaded</span>
    </div>
    <p class="hint">
      Download the embedding model (~{status.size_mb} MB) to enable semantic search across your library.
    </p>
    <button class="download-btn" on:click={startDownload}>
      Download Model
    </button>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .model-manager {
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
  }

  h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text);
  }

  .status-available, .status-missing {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.95rem;
  }

  .status-available {
    color: #16a34a;
  }

  .status-available .status-icon {
    font-size: 1.1rem;
  }

  .status-missing {
    color: var(--text-dim);
  }

  .status-loading {
    color: var(--text-dim);
    font-size: 0.95rem;
  }

  .hint {
    margin: 0.75rem 0;
    font-size: 0.9rem;
    color: var(--text-dim);
  }

  .download-btn {
    padding: 0.6rem 1.2rem;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--accent);
    color: white;
    font-size: 0.9rem;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .download-btn:hover {
    opacity: 0.9;
  }

  .download-progress {
    margin: 1rem 0;
  }

  .progress-bar {
    width: 100%;
    height: 8px;
    background: var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }

  .progress-text {
    margin-top: 0.5rem;
    font-size: 0.85rem;
    color: var(--text-dim);
  }

  .error {
    margin-top: 1rem;
    color: #dc2626;
    font-size: 0.9rem;
  }
</style>

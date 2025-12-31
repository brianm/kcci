import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface Stats {
    total_books: number;
    enriched: number;
    with_embeddings: number;
}

export interface Book {
    asin: string;
    title: string;
    authors: string[];
    cover_url: string | null;
    percent_read: number;
    resource_type: string | null;
    origin_type: string | null;
    description: string | null;
    subjects: string[];
    publish_year: number | null;
    isbn: string | null;
    openlibrary_key: string | null;
    distance: number | null;
    rank: number | null;
}

export interface PaginatedBooks {
    books: Book[];
    page: number;
    per_page: number;
    total: number;
    total_pages: number;
}

export interface SyncProgress {
    stage: string;
    message: string;
    current: number | null;
    total: number | null;
}

export interface SyncStats {
    imported: number;
    enriched: number;
    embedded: number;
}

export async function getStats(): Promise<Stats> {
    return invoke('get_stats');
}

export async function search(query: string, mode: 'semantic' | 'fts', limit?: number): Promise<Book[]> {
    return invoke('search', { query, mode, limit });
}

export async function getBook(asin: string): Promise<Book | null> {
    return invoke('get_book', { asin });
}

export interface Filter {
    field: 'title' | 'author' | 'description' | 'subject';
    op: 'contains' | 'has';
    value: string;
}

export interface ListBooksOptions {
    page?: number;
    perPage?: number;
    sortBy?: 'title' | 'author' | 'year';
    sortDir?: 'asc' | 'desc';
    filters?: Filter[];
}

export async function listBooks(options: ListBooksOptions = {}): Promise<PaginatedBooks> {
    return invoke('list_books', options);
}

export async function getSubjects(): Promise<string[]> {
    return invoke('get_subjects');
}

export async function clearMetadata(): Promise<number> {
    return invoke('clear_metadata');
}

export interface ModelStatus {
    available: boolean;
    size_mb: number;
}

export interface DownloadProgress {
    bytes_downloaded: number;
    total_bytes: number;
    percent: number;
    file: string;
}

export async function getModelStatus(): Promise<ModelStatus> {
    return invoke('get_model_status');
}

export async function downloadModel(
    onProgress?: (progress: DownloadProgress) => void
): Promise<void> {
    let unlisten: UnlistenFn | null = null;

    if (onProgress) {
        unlisten = await listen<DownloadProgress>('model-download-progress', (event) => {
            onProgress(event.payload);
        });
    }

    try {
        await invoke('download_model');
    } finally {
        if (unlisten) {
            unlisten();
        }
    }
}

export async function syncLibrary(
    webarchivePath?: string,
    onProgress?: (progress: SyncProgress) => void
): Promise<SyncStats> {
    let unlisten: UnlistenFn | null = null;

    if (onProgress) {
        unlisten = await listen<SyncProgress>('sync-progress', (event) => {
            onProgress(event.payload);
        });
    }

    try {
        return await invoke('sync_library', { webarchivePath });
    } finally {
        if (unlisten) {
            unlisten();
        }
    }
}

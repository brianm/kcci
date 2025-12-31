<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Filter } from '../lib/api';

  export let subjects: string[] = [];

  interface FilterRow {
    id: string;
    field: Filter['field'];
    op: Filter['op'];
    value: string;
  }

  let filterRows: FilterRow[] = [];
  let debounceTimer: ReturnType<typeof setTimeout>;

  const dispatch = createEventDispatcher<{ change: Filter[] }>();

  function generateId(): string {
    return Math.random().toString(36).substring(2, 9);
  }

  function addFilter() {
    filterRows = [...filterRows, {
      id: generateId(),
      field: 'title',
      op: 'contains',
      value: ''
    }];
  }

  function removeFilter(id: string) {
    filterRows = filterRows.filter(f => f.id !== id);
    emitChange();
  }

  function getOperationsForField(field: Filter['field']): { value: Filter['op']; label: string }[] {
    if (field === 'subject') {
      return [{ value: 'has', label: 'has' }];
    }
    return [{ value: 'contains', label: 'contains' }];
  }

  function handleFieldChange(row: FilterRow) {
    // Reset operation and value when field changes
    const ops = getOperationsForField(row.field);
    row.op = ops[0].value;
    row.value = '';
    filterRows = filterRows;
    emitChange();
  }

  function handleValueChange() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(emitChange, 300);
  }

  function emitChange() {
    const validFilters: Filter[] = filterRows
      .filter(row => row.value.trim() !== '')
      .map(row => ({
        field: row.field,
        op: row.op,
        value: row.value.trim()
      }));
    dispatch('change', validFilters);
  }

  function handleSubjectSelect(row: FilterRow, event: Event) {
    const select = event.target as HTMLSelectElement;
    row.value = select.value;
    filterRows = filterRows;
    emitChange();
  }
</script>

<div class="filter-builder">
  {#each filterRows as row (row.id)}
    <div class="filter-row">
      <select
        bind:value={row.field}
        on:change={() => handleFieldChange(row)}
        class="field-select"
      >
        <option value="title">Title</option>
        <option value="author">Author</option>
        <option value="description">Description</option>
        <option value="subject">Subject</option>
      </select>

      <select bind:value={row.op} class="op-select" on:change={emitChange}>
        {#each getOperationsForField(row.field) as op}
          <option value={op.value}>{op.label}</option>
        {/each}
      </select>

      {#if row.field === 'subject'}
        <select
          value={row.value}
          on:change={(e) => handleSubjectSelect(row, e)}
          class="value-input"
        >
          <option value="">Select subject...</option>
          {#each subjects as subject}
            <option value={subject}>{subject}</option>
          {/each}
        </select>
      {:else}
        <input
          type="text"
          bind:value={row.value}
          on:input={handleValueChange}
          placeholder="Enter value..."
          class="value-input"
        />
      {/if}

      <button class="remove-btn" on:click={() => removeFilter(row.id)} title="Remove filter">
        &times;
      </button>
    </div>
  {/each}

  <button class="add-btn" on:click={addFilter}>
    + Add Filter
  </button>
</div>

<style>
  .filter-builder {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .filter-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .field-select {
    width: 120px;
  }

  .op-select {
    width: 100px;
  }

  .value-input {
    flex: 1;
    min-width: 150px;
  }

  .filter-row select,
  .filter-row input {
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.9rem;
  }

  .filter-row select:focus,
  .filter-row input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .remove-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text-dim);
    font-size: 1.2rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .remove-btn:hover {
    background: var(--border);
    color: var(--text);
  }

  .add-btn {
    align-self: flex-start;
    padding: 0.4rem 0.8rem;
    border: 1px dashed var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text-dim);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .add-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>

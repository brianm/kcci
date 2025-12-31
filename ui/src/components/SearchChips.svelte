<script lang="ts">
  import type { SearchFilter } from '../lib/api';

  interface Props {
    filters: SearchFilter[];
    onchange?: (filters: SearchFilter[]) => void;
    placeholder?: string;
    inputElement?: HTMLInputElement;
  }

  let { filters = $bindable([]), onchange, placeholder = 'Filter...', inputElement = $bindable() }: Props = $props();

  const FIELD_PREFIXES = ['all:', 'title:', 'author:', 'description:', 'subject:'];

  let inputValue = $state('');
  let showSuggestions = $state(false);
  let selectedSuggestionIndex = $state(0);

  // Get matching prefix suggestions based on input
  let suggestions = $derived.by(() => {
    if (!inputValue || inputValue.includes(':')) return [];
    const lower = inputValue.toLowerCase();
    return FIELD_PREFIXES.filter(p => p.startsWith(lower));
  });

  function parseInput(input: string): SearchFilter | null {
    const trimmed = input.trim();
    if (!trimmed) return null;

    const colonIndex = trimmed.indexOf(':');
    if (colonIndex > 0) {
      const field = trimmed.slice(0, colonIndex).toLowerCase();
      const value = trimmed.slice(colonIndex + 1).trim();
      if (value && ['all', 'title', 'author', 'description', 'subject'].includes(field)) {
        return { field, value };
      }
    }
    // No valid prefix, treat as "all"
    return { field: 'all', value: trimmed };
  }

  function addChip() {
    const filter = parseInput(inputValue);
    if (filter) {
      filters = [...filters, filter];
      inputValue = '';
      showSuggestions = false;
      onchange?.(filters);
    }
  }

  function removeChip(index: number) {
    filters = filters.filter((_, i) => i !== index);
    onchange?.(filters);
  }

  function editChip(index: number) {
    const filter = filters[index];
    inputValue = formatChip(filter);
    removeChip(index);
    inputElement?.focus();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === 'Tab') {
      if (showSuggestions && suggestions.length > 0) {
        // Complete the suggestion
        e.preventDefault();
        inputValue = suggestions[selectedSuggestionIndex];
        showSuggestions = false;
      } else if (inputValue.trim()) {
        e.preventDefault();
        addChip();
      }
    } else if (e.key === 'Backspace' && !inputValue && filters.length > 0) {
      // Remove last chip when backspacing on empty input
      removeChip(filters.length - 1);
    } else if (e.key === 'ArrowDown' && showSuggestions) {
      e.preventDefault();
      selectedSuggestionIndex = Math.min(selectedSuggestionIndex + 1, suggestions.length - 1);
    } else if (e.key === 'ArrowUp' && showSuggestions) {
      e.preventDefault();
      selectedSuggestionIndex = Math.max(selectedSuggestionIndex - 1, 0);
    } else if (e.key === 'Escape') {
      showSuggestions = false;
    }
  }

  function handleInput() {
    showSuggestions = suggestions.length > 0;
    selectedSuggestionIndex = 0;
  }

  function selectSuggestion(suggestion: string) {
    inputValue = suggestion;
    showSuggestions = false;
    inputElement?.focus();
  }

  function handleContainerClick() {
    inputElement?.focus();
  }

  function formatChip(filter: SearchFilter): string {
    return `${filter.field}:${filter.value}`;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="search-chips-container" onclick={handleContainerClick} role="textbox" tabindex="-1">
  {#each filters as filter, index}
    <button class="chip" type="button" ondblclick={() => editChip(index)} title="Double-click to edit">
      <span class="chip-text">{formatChip(filter)}</span>
      <span class="chip-remove" onclick={() => removeChip(index)} role="button" tabindex="-1">×</span>
    </button>
  {/each}

  <div class="input-wrapper">
    <input
      type="text"
      bind:this={inputElement}
      bind:value={inputValue}
      oninput={handleInput}
      onkeydown={handleKeydown}
      onfocus={() => showSuggestions = suggestions.length > 0}
      onblur={() => setTimeout(() => showSuggestions = false, 150)}
      {placeholder}
      class="chip-input"
    />

    {#if showSuggestions && suggestions.length > 0}
      <div class="suggestions">
        {#each suggestions as suggestion, i}
          <button
            class="suggestion"
            class:selected={i === selectedSuggestionIndex}
            onmousedown={() => selectSuggestion(suggestion)}
            type="button"
          >
            {suggestion}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .search-chips-container {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    height: 38px;
    padding: 0 0.75rem;
    background: var(--bg-light);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: text;
  }

  .search-chips-container:focus-within {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-light);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    padding: 0.15rem 0.4rem;
    background: var(--accent-light);
    border: 1px solid var(--accent);
    border-radius: 4px;
    font-size: 0.8rem;
    color: var(--accent);
    cursor: pointer;
  }

  .chip:hover {
    background: var(--accent);
    color: white;
  }

  .chip:hover .chip-remove {
    color: white;
  }

  .chip-text {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--accent);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    border-radius: 2px;
  }

  .chip-remove:hover {
    background: var(--accent);
    color: white;
  }

  .input-wrapper {
    position: relative;
    flex: 1;
    min-width: 120px;
  }

  .chip-input {
    width: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.95rem;
    line-height: 1.4;
    outline: none;
  }

  .chip-input::placeholder {
    color: var(--text-dim);
  }

  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 0.25rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    z-index: 100;
    overflow: hidden;
  }

  .suggestion {
    display: block;
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.9rem;
    text-align: left;
    cursor: pointer;
  }

  .suggestion:hover,
  .suggestion.selected {
    background: var(--accent-light);
    color: var(--accent);
  }
</style>

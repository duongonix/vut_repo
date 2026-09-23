<script lang="ts">
  import { Search, X, ArrowDown, ArrowUp, CornerDownLeft, FileText } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import { tick } from 'svelte';
  import { searchDocs, type SearchResult } from '$lib/search/pagefind';
  let { open = $bindable(false) }: { open?: boolean } = $props();
  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let query = $state('');
  let results = $state<SearchResult[]>([]);
  let selected = $state(0);
  let loading = $state(false);
  let error = $state('');

  $effect(() => {
    if (open && dialog && !dialog.open) {
      dialog.showModal();
      tick().then(() => input?.focus());
    } else if (!open && dialog?.open) dialog.close();
  });
  $effect(() => {
    const value = query.trim();
    let current = true;
    results = [];
    selected = 0;
    error = '';
    loading = !!value;
    const timer = setTimeout(async () => {
      if (!value) return;
      try {
        const found = await searchDocs(value);
        if (current) results = found;
      } catch {
        if (current)
          error = 'Search is unavailable. Please try again, or browse the documentation.';
      } finally {
        if (current) loading = false;
      }
    }, 180);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  });
  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      // Search inputs can consume Escape to clear text before dialog cancellation.
      event.preventDefault();
      open = false;
    } else if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && results.length) {
      event.preventDefault();
      selected =
        (selected + (event.key === 'ArrowDown' ? 1 : -1) + results.length) % results.length;
      document.getElementById(`search-result-${selected}`)?.scrollIntoView({ block: 'nearest' });
    } else if (event.key === 'Enter' && results[selected] && event.target === input) {
      event.preventDefault();
      open = false;
      goto(results[selected].url);
    }
  }
</script>

<dialog
  bind:this={dialog}
  aria-label="Search documentation"
  onclose={() => (open = false)}
  onkeydown={keydown}
>
  <div class="search-field">
    <Search size={20} /><input
      bind:this={input}
      bind:value={query}
      type="search"
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={results.length > 0}
      placeholder="Search the Vut docs..."
      aria-label="Search query"
      aria-controls={results.length ? 'search-results' : undefined}
      aria-activedescendant={results.length ? `search-result-${selected}` : undefined}
      autocomplete="off"
    /><button class="icon-button" aria-label="Close search" onclick={() => (open = false)}
      ><X size={18} /></button
    >
  </div>
  <div class="results">
    {#if !query.trim()}<div class="empty">
        {@render BookPrompt()}
        <p>What would you like to build?</p>
        <span>Search for variables, functions, the CLI, and more.</span>
        <div class="suggestions">
          {#each ['Variables', 'Installation', 'Functions'] as suggestion}<button
              onclick={() => (query = suggestion)}>{suggestion}</button
            >{/each}
        </div>
      </div>
    {:else if loading}<p class="status" role="status">Searching documentation…</p>
    {:else if error}<div class="status" role="status">
        <p>{error}</p>
        <a href="/docs/getting-started/introduction/" onclick={() => (open = false)}
          >Browse documentation →</a
        >
      </div>
    {:else if !results.length}<p class="status" role="status">
        No results for “{query}”. Try another word.
      </p>
    {:else}
      <p class="result-count" role="status">{results.length} matching sections</p>
      <div class="result-list" id="search-results" role="listbox" aria-label="Search results">
        {#each results as result, index}<a
            role="option"
            aria-selected={selected === index}
            tabindex={selected === index ? 0 : -1}
            id={`search-result-${index}`}
            class:selected={selected === index}
            href={result.url}
            onclick={() => (open = false)}
            onfocus={() => (selected = index)}
            ><FileText size={18} />
            <div>
              <small
                >{result.meta.section || 'Documentation'} <span>›</span> {result.meta.title}</small
              ><strong>{result.title || result.meta.title}</strong>
              <p>{@html result.excerpt}</p>
            </div>
            <CornerDownLeft size={15} /></a
          >{/each}
      </div>
    {/if}
  </div>
  <div class="search-footer">
    <span><ArrowUp size={12} /><ArrowDown size={12} /> to navigate</span><span
      ><CornerDownLeft size={12} /> to open</span
    ><span><kbd>esc</kbd> to close</span><span class="powered">Local search · Pagefind</span>
  </div>
</dialog>

{#snippet BookPrompt()}<div class="search-symbol">
    <Search size={27} strokeWidth={1.3} />
  </div>{/snippet}

<style>
  dialog {
    padding: 0;
    border-radius: 15px;
    width: min(640px, calc(100% - 32px));
    margin: 13vh auto auto;
    box-shadow: 0 25px 100px #0005;
  }
  .search-field {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 17px 20px;
    border-bottom: 1px solid var(--border);
    color: var(--primary);
  }
  input {
    min-width: 0;
    flex: 1;
    background: none;
    border: 0;
    outline: 0;
    color: var(--text);
    font-size: 16px;
    padding: 5px 0;
  }
  input:focus-visible {
    outline: 0;
  }
  .results {
    min-height: 220px;
    max-height: 55vh;
    overflow-y: auto;
    padding: 10px;
  }
  .empty {
    text-align: center;
    padding: 20px 8px;
  }
  .empty p {
    color: var(--text);
    font-size: 15px;
    margin: 15px 0 5px;
  }
  .empty > span {
    color: var(--subtle);
    font-size: 12px;
  }
  .search-symbol {
    display: inline-flex;
    padding: 13px;
    border-radius: 12px;
    background: #7868ff10;
    color: var(--primary);
  }
  .suggestions {
    display: flex;
    justify-content: center;
    gap: 8px;
    margin-top: 22px;
  }
  .suggestions button {
    color: var(--muted);
    background: var(--hover);
    padding: 7px 11px;
    border-radius: 6px;
    font-size: 11px;
  }
  .status {
    padding: 30px 20px;
    font-size: 14px;
  }
  .status a {
    color: var(--primary);
  }
  .result-count {
    margin: 3px 10px 10px;
    font-size: 11px;
  }
  .result-list > a {
    display: flex;
    align-items: center;
    gap: 13px;
    padding: 14px;
    border-radius: 9px;
    border: 1px solid transparent;
  }
  .result-list > a.selected {
    background: #7868ff12;
    border-color: #7868ff33;
  }
  .result-list > a > :global(svg) {
    color: var(--subtle);
  }
  .result-list > a > :global(svg:last-child) {
    margin-left: auto;
  }
  small {
    display: block;
    color: var(--subtle);
    font-size: 10px;
    margin-bottom: 6px;
  }
  small span {
    margin-inline: 4px;
  }
  strong {
    font-size: 14px;
    font-weight: 550;
  }
  .results a p {
    font-size: 12px;
    margin: 5px 0 0;
    line-height: 1.6;
  }
  .results :global(mark) {
    background: #7868ff24;
    color: var(--primary);
    border-radius: 2px;
  }
  .search-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 15px;
    padding: 13px 20px;
    border-top: 1px solid var(--border);
    color: var(--subtle);
    font-size: 10px;
  }
  .search-footer span {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }
  .powered {
    margin-left: auto;
  }
  @media (max-width: 480px) {
    .powered {
      display: none !important;
    }
    .search-field {
      padding-inline: 14px;
    }
    input {
      font-size: 14px;
    }
  }
</style>

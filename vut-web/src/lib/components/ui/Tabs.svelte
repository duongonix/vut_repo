<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    labels,
    label,
    content,
    active = $bindable(0)
  }: {
    labels: string[];
    label: string;
    content: Snippet<[number]>;
    active?: number;
  } = $props();
  const id = $props.id();
  let tablist: HTMLDivElement;
  function navigate(event: KeyboardEvent, index: number) {
    const next =
      event.key === 'ArrowRight'
        ? (index + 1) % labels.length
        : event.key === 'ArrowLeft'
          ? (index + labels.length - 1) % labels.length
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? labels.length - 1
              : undefined;
    if (next === undefined) return;
    event.preventDefault();
    active = next;
    tablist.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={label} bind:this={tablist}>
  {#each labels as name, index}
    <button
      type="button"
      role="tab"
      id={`${id}-tab-${index}`}
      aria-controls={`${id}-panel`}
      aria-selected={active === index}
      tabindex={active === index ? 0 : -1}
      onclick={() => (active = index)}
      onkeydown={(event) => navigate(event, index)}>{name}</button
    >
  {/each}
</div>
<div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-tab-${active}`} tabindex="0">
  {@render content(active)}
</div>

<style>
  .tabs {
    display: flex;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
    padding: 5px;
    gap: 3px;
  }
  button {
    flex: 1;
    white-space: nowrap;
    color: var(--subtle);
    background: transparent;
    border-radius: 8px;
    font-size: 12px;
    padding: 11px 8px;
  }
  button[aria-selected='true'] {
    color: var(--text);
    background: var(--hover);
  }
  button:focus-visible {
    outline-offset: -3px;
  }
</style>

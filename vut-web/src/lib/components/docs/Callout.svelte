<script lang="ts">
  import { Info, Lightbulb, TriangleAlert, CircleAlert } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  let {
    kind = 'note',
    title,
    children
  }: {
    kind?: 'note' | 'tip' | 'warning' | 'important';
    title?: string;
    children: Snippet;
  } = $props();
  const icons = { note: Info, tip: Lightbulb, warning: TriangleAlert, important: CircleAlert };
  const Icon = $derived(icons[kind]);
</script>

<aside class="callout" data-kind={kind}>
  <div><Icon size={17} /><strong>{title || kind}</strong></div>
  <section>{@render children()}</section>
</aside>

<style>
  .callout {
    border: 1px solid #7868ff35;
    border-left: 3px solid var(--primary);
    background: #7868ff0b;
    padding: 17px 20px;
    border-radius: 8px;
    margin-block: 25px;
    font-size: 14px;
  }
  .callout > div {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--primary);
    margin-bottom: 8px;
  }
  strong {
    text-transform: capitalize;
    font-size: 13px;
    font-weight: 550;
  }
  .callout[data-kind='warning'] {
    border-color: #d49a4544;
    border-left-color: #d49a45;
    background: #d49a4509;
  }
  .callout[data-kind='warning'] > div {
    color: #d49a45;
  }
</style>

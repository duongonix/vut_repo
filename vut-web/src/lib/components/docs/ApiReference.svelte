<script lang="ts">
  import type { Snippet } from 'svelte';
  let {
    signature,
    parameters,
    returns,
    errors,
    children
  }: {
    signature: string;
    parameters: string;
    returns: string;
    errors?: string;
    children?: Snippet;
  } = $props();
</script>

<section class="api">
  <code>{signature}</code>
  <dl>
    <dt>Parameters</dt>
    <dd>{parameters}</dd>
    <dt>Returns</dt>
    <dd>{returns}</dd>
    {#if errors}<dt>Errors</dt>
      <dd>{errors}</dd>{/if}
  </dl>
  {#if children}{@render children()}{/if}
</section>

<style>
  .api {
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    margin-block: 24px;
  }
  code {
    font-family: 'Geist Mono Variable', monospace;
    color: var(--primary);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  dt {
    color: var(--text);
    font-size: 13px;
    margin-top: 15px;
    font-weight: 550;
  }
  dd {
    font-size: 13px;
    margin-left: 0;
  }
</style>

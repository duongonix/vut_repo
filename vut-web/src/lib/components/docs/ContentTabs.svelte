<script lang="ts">
  import Tabs from '../ui/Tabs.svelte';
  let { label, items }: { label: string; items: { label: string; text: string; code?: string }[] } =
    $props();
</script>

<Tabs labels={items.map((item) => item.label)} {label}>
  {#snippet content(index)}
    {@const item = items[index]}
    <div class="content">
      {#if item.code}
        <div class="code-block">
          <div class="code-toolbar">
            <span>{item.label}</span><button data-copy-code aria-label="Copy code">Copy</button>
          </div>
          <pre class="shiki"><code>{item.code}</code></pre>
        </div>
      {/if}
      <p>{item.text}</p>
    </div>
  {/snippet}
</Tabs>

<style>
  .content {
    padding-top: 18px;
  }
  .code-block {
    margin-top: 0;
  }
</style>

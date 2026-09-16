<script lang="ts">
  import { Play } from '@lucide/svelte';
  import { codeCopy } from '$lib/actions/code-copy';
  let {
    html,
    filename = 'main.vut',
    run = false
  }: { html: string; filename?: string; run?: boolean } = $props();
</script>

<div class="window code-block" use:codeCopy>
  <div class="code-toolbar">
    <div class="dots" aria-hidden="true"><i></i><i></i><i></i></div>
    <span>{filename}</span>
    <div class="controls">
      <button type="button" data-copy-code aria-label="Copy code">Copy</button>{#if run}<a
          href="/playground/"
          class="run">Try it <Play size={12} fill="currentColor" /></a
        >{/if}
    </div>
  </div>
  {@html html}
</div>

<style>
  .window {
    margin: 0;
    background: color-mix(in srgb, var(--surface) 94%, transparent);
    box-shadow: 0 20px 80px #02040a35;
  }
  .code-toolbar {
    border-bottom: 0;
    min-height: 49px;
    gap: 18px;
  }
  .dots {
    display: flex;
    gap: 6px;
  }
  i {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #fb665e;
  }
  i:nth-child(2) {
    background: #f6c349;
  }
  i:nth-child(3) {
    background: #51c76a;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-left: auto;
  }
  .run {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--text);
    padding: 7px 12px;
    background: var(--hover);
    border: 1px solid var(--border);
    border-radius: 18px;
    font-family: 'Geist Variable', sans-serif;
  }
  .window :global(.shiki) {
    padding-top: 10px;
    padding-bottom: 25px;
    font-size: 13px;
  }
  @media (max-width: 400px) {
    .code-toolbar {
      gap: 8px;
      padding-inline: 12px;
    }
    .controls {
      gap: 2px;
    }
  }
</style>

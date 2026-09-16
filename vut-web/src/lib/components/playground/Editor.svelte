<script lang="ts">
  import { Copy, RotateCcw, Play, Download, Terminal } from '@lucide/svelte';
  import { hello, examples } from '$lib/content/examples';
  let code = $state(hello);
  let selected = $state('Hello Vut');
  let copied = $state('Copy');
  let timer: ReturnType<typeof setTimeout>;
  const options = [{ label: 'Hello Vut', code: hello }, ...examples];
  $effect(() => () => clearTimeout(timer));
  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      copied = 'Copied';
    } catch {
      copied = 'Copy failed';
    }
    clearTimeout(timer);
    timer = setTimeout(() => (copied = 'Copy'), 1800);
  }
  function download() {
    const url = URL.createObjectURL(new Blob([code], { type: 'text/plain' }));
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = 'main.vut';
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
</script>

<div class="playground">
  <div class="toolbar">
    <span class="filename"><span></span>main.vut</span><label class="sr-only" for="example-select"
      >Choose example</label
    ><select
      id="example-select"
      bind:value={selected}
      onchange={() => (code = options.find((option) => option.label === selected)!.code)}
      >{#each options as option}<option>{option.label}</option>{/each}</select
    >
    <div class="buttons">
      <button
        class="icon-button"
        aria-label="Reset example"
        onclick={() => (code = options.find((option) => option.label === selected)!.code)}
        ><RotateCcw size={16} /></button
      ><button class="icon-button" aria-label="Download main.vut" onclick={download}
        ><Download size={16} /></button
      ><button class="copy" onclick={copy}><Copy size={14} />{copied}</button><button
        class="run"
        disabled
        aria-describedby="runtime-status"><Play size={13} />Run</button
      >
    </div>
  </div>
  <div class="panels">
    <div class="editor">
      <label class="sr-only" for="vut-editor">Vut source code</label><textarea
        id="vut-editor"
        bind:value={code}
        spellcheck="false"
        autocapitalize="off"
        autocomplete="off"
        aria-describedby="editor-help"></textarea>
      <p id="editor-help">Vut · UTF-8 · 2-space indentation</p>
    </div>
    <section class="output" aria-label="Program output">
      <div class="output-title"><Terminal size={15} />Output</div>
      <div class="runtime-note">
        <Terminal size={30} strokeWidth={1.2} />
        <h2>Bring your ideas to life.</h2>
        <p id="runtime-status">
          Browser execution is not available yet. Download your code and run it with the Vut CLI.
        </p>
        <code>vut run main.vut</code><a href="/docs/getting-started/installation/" class="text-link"
          >Set up Vut →</a
        >
      </div>
    </section>
  </div>
</div>

<style>
  .playground {
    border: 1px solid var(--border-hover);
    border-radius: 14px;
    overflow: hidden;
    background: var(--surface);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 20px;
    border-bottom: 1px solid var(--border);
    padding: 12px 20px;
  }
  .filename {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: 'Geist Mono Variable', monospace;
    font-size: 12px;
  }
  .filename span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--primary);
  }
  select {
    font-size: 12px;
    color: var(--muted);
    background: var(--hover);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 7px 9px;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-left: auto;
  }
  .copy,
  .run {
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--hover);
    font-size: 12px;
    border-radius: 6px;
    padding: 9px 12px;
  }
  .run {
    background: var(--gradient);
    color: white;
  }
  .panels {
    display: grid;
    grid-template-columns: 1.15fr 1fr;
    min-height: 430px;
  }
  .editor {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  textarea {
    width: 100%;
    flex: 1;
    min-height: 365px;
    resize: vertical;
    padding: 25px;
    border: 0;
    background: transparent;
    color: var(--text);
    line-height: 1.9;
    font-size: 13px;
    font-family: 'Geist Mono Variable', monospace;
    tab-size: 2;
  }
  textarea:focus-visible {
    outline-offset: -3px;
  }
  .editor > p {
    font-size: 10px;
    margin: 0;
    padding: 10px 22px;
    border-top: 1px solid var(--border);
  }
  .output {
    border-left: 1px solid var(--border);
    background: var(--secondary);
  }
  .output-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--subtle);
    padding: 17px 20px;
    border-bottom: 1px solid var(--border);
  }
  .runtime-note {
    padding: 48px 30px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }
  .runtime-note > :global(svg) {
    color: var(--primary);
    margin-bottom: 20px;
  }
  h2 {
    font-size: 20px;
  }
  .runtime-note p {
    font-size: 13px;
    max-width: 300px;
  }
  code {
    font-family: 'Geist Mono Variable', monospace;
    font-size: 12px;
    padding: 10px 15px;
    border: 1px solid var(--border);
    border-radius: 6px;
    margin: 7px 0 24px;
  }
  @media (max-width: 760px) {
    .panels {
      grid-template-columns: 1fr;
    }
    .output {
      border-left: 0;
      border-top: 1px solid var(--border);
    }
    .toolbar {
      flex-wrap: wrap;
      gap: 10px;
      padding: 12px;
    }
    .buttons {
      gap: 3px;
    }
    .runtime-note {
      padding-block: 28px;
    }
  }
</style>

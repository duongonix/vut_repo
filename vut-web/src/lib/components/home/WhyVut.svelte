<script lang="ts">
  import { Check, ArrowRight } from '@lucide/svelte';
  import { codeCopy } from '$lib/actions/code-copy';
  import Tabs from '../ui/Tabs.svelte';
  let { examples }: { examples: { label: string; code: string; html: string }[] } = $props();
</script>

<section class="why container">
  <div class="copy">
    <p class="eyebrow">Why Vut</p>
    <h2 class="section-heading">A language for<br />real possibilities.</h2>
    <p class="description">
      Vut combines simplicity, performance, and a modern developer experience — so you can focus on
      what matters: building amazing things.
    </p>
    <ul>
      {#each ['Modern and expressive syntax', 'Strict static typing', 'Native-oriented performance', 'Cross-platform by design', 'Modern package tooling'] as item}<li
        >
          <span><Check size={11} strokeWidth={3} /></span>{item}
        </li>{/each}
    </ul>
    <a class="button primary" href="/docs/getting-started/introduction/"
      >Learn more <ArrowRight size={16} /></a
    >
  </div>
  <div class="showcase code-block" use:codeCopy>
    <Tabs labels={examples.map((example) => example.label)} label="Vut examples">
      {#snippet content(index)}{@html examples[index].html}{/snippet}
    </Tabs>
    <div class="showcase-bottom">
      <span>Just works.</span><button data-copy-code aria-label="Copy example code">Copy</button>
    </div>
  </div>
</section>

<style>
  .why {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 90px;
    padding-block: 95px 65px;
    align-items: center;
  }
  .eyebrow {
    margin-bottom: 18px;
  }
  .section-heading {
    margin-bottom: 18px;
  }
  .description {
    font-size: 16px;
    max-width: 490px;
  }
  ul {
    padding: 0;
    list-style: none;
    display: grid;
    gap: 12px;
    margin: 23px 0 30px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--muted);
  }
  li span {
    display: grid;
    place-items: center;
    width: 17px;
    height: 17px;
    background: var(--gradient);
    border-radius: 50%;
    color: white;
  }
  .showcase {
    margin: 0;
    min-width: 0;
    background: linear-gradient(145deg, var(--surface), var(--secondary));
  }
  .showcase :global(.shiki) {
    min-height: 292px;
    font-size: 13px;
  }
  .showcase-bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 25px 22px;
  }
  .showcase-bottom span {
    font-family: cursive;
    font-size: 27px;
    font-style: italic;
    transform: rotate(-6deg);
    color: var(--primary);
  }
  .showcase-bottom button {
    background: transparent;
    color: var(--subtle);
    font-size: 12px;
    padding: 7px;
  }
  @media (max-width: 1100px) {
    .why {
      gap: 40px;
    }
  }
  @media (max-width: 800px) {
    .why {
      grid-template-columns: 1fr;
      gap: 35px;
      padding-top: 65px;
    }
  }
</style>

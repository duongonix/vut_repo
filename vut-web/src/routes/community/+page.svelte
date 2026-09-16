<script lang="ts">
  import { ArrowRight, BookOpen, Code, MessageCircle } from '@lucide/svelte';
  import Seo from '$lib/components/layout/Seo.svelte';
  import { site } from '$lib/config/site';
</script>

<Seo
  title="Community — Vut"
  description="Help shape an open and expressive programming language."
/>
<main id="main-content" class="standalone container">
  <p class="eyebrow">Built in the open</p>
  <h1>A brighter future.<br />Built together.</h1>
  <p class="intro">
    Good languages grow through shared ideas. Explore Vut, experiment with the syntax, and help make
    the developer experience better.
  </p>
  <div class="cards">
    <a href="/docs/getting-started/introduction/"
      ><BookOpen size={25} />
      <h2>Get to know Vut</h2>
      <p>Start with the language and its design.</p>
      <ArrowRight size={16} /></a
    ><a href="/playground/"
      ><Code size={25} />
      <h2>Try an idea</h2>
      <p>Explore examples and make them your own.</p>
      <ArrowRight size={16} /></a
    >
    <div>
      <MessageCircle size={25} />
      <h2>Find the conversation</h2>
      {#if site.community}<a href={site.community} class="text-link"
          >Join the community <ArrowRight size={15} /></a
        >{:else}<p>Official community channels will be linked here when they are announced.</p>{/if}
    </div>
  </div>
  {#if site.github}<a class="button primary" href={site.github}
      >Explore on GitHub <ArrowRight size={15} /></a
    >{/if}
</main>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 18px;
    margin-block: 40px;
  }
  .cards > * {
    padding: 25px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
  }
  .cards :global(svg) {
    color: var(--primary);
  }
  h2 {
    font-size: 18px;
    margin: 20px 0 10px;
  }
  .cards p {
    font-size: 13px;
  }
  @media (max-width: 700px) {
    .cards {
      grid-template-columns: 1fr;
    }
  }
</style>

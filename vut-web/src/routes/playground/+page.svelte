<script lang="ts">
  import Seo from '$lib/components/layout/Seo.svelte';
  import Editor from '$lib/components/playground/Editor.svelte';
</script>

<Seo
  title="Playground — Vut"
  description="Explore Vut examples, edit code, and take your next idea to the native toolchain."
/>
<main id="main-content" class="container playground-page">
  <p class="eyebrow">A little room to experiment</p>
  <h1>Make something <span>yours.</span></h1>
  <p>Explore the language. Edit an example. Take it from here.</p>
  <Editor />
</main>

<style>
  .playground-page {
    padding-block: 65px 80px;
  }
  h1 {
    font-size: clamp(35px, 4vw, 52px);
    margin-bottom: 16px;
  }
  h1 span {
    color: var(--primary);
  }
  main > p:not(.eyebrow) {
    margin-bottom: 35px;
    font-size: 16px;
  }
</style>

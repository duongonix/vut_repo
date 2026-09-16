<script lang="ts">
  import Seo from '$lib/components/layout/Seo.svelte';
  import { site } from '$lib/config/site';
</script>

<Seo title="Changelog — Vut" description="Follow language, compiler, and tooling development." />
<main id="main-content" class="standalone container">
  <p class="eyebrow">The road ahead</p>
  <h1>Always moving forward.</h1>
  <p class="intro">
    Language updates, compiler improvements, and better tools. A home for what changes in Vut.
  </p>
  <section>
    <span>{site.versions[0].label}</span>
    <h2>Under active development</h2>
    <p>
      The repository currently identifies as version 0.1.0. A verified release history has not been
      published in this documentation yet.
    </p>
    <p>
      Release notes will cover language changes, compiler and tooling updates, and breaking changes
      as they are confirmed.
    </p>
    <a class="text-link" href="/docs/getting-started/introduction/"
      >Explore the current documentation →</a
    >
  </section>
</main>

<style>
  section {
    margin-top: 40px;
    border-left: 1px solid var(--border-hover);
    padding: 10px 0 15px 28px;
    max-width: 680px;
  }
  section > span {
    color: var(--primary);
    font-size: 12px;
  }
  h2 {
    margin-top: 18px;
    font-size: 26px;
  }
  section p {
    font-size: 14px;
  }
</style>

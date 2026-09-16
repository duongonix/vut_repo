<script lang="ts">
  import { page } from '$app/state';
  import Seo from '$lib/components/layout/Seo.svelte';
</script>

<Seo title={`${page.status} — Vut`} description="Find your way back to the Vut documentation." />
<main id="main-content" class="error container">
  <p class="eyebrow">A small detour</p>
  <span>{page.status}</span>
  <h1>{page.status === 404 ? "This path doesn't exist." : 'Something went wrong.'}</h1>
  <p>There is still plenty to explore. Let’s get you back on track.</p>
  <a class="button primary" href="/docs/getting-started/introduction/">Back to Docs →</a>
</main>

<style>
  .error {
    text-align: center;
    padding-block: 85px 110px;
    min-height: 65vh;
  }
  .error > span {
    font-size: 110px;
    font-weight: 650;
    color: var(--primary);
    letter-spacing: -0.08em;
  }
  h1 {
    font-size: 34px;
    margin-block: 20px;
  }
  .button {
    margin-top: 20px;
  }
</style>

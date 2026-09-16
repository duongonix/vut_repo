<script lang="ts">
  import { page } from '$app/state';
  import { site } from '$lib/config/site';
  let { title = 'Vut — Build a brighter tomorrow', description = site.description } = $props<{
    title?: string;
    description?: string;
  }>();
  // A static preview has no public origin. Never publish SvelteKit's synthetic origin.
  const canonical = $derived(
    site.origin ? new URL(page.url.pathname, site.origin).href : undefined
  );
</script>

<svelte:head>
  <link rel="icon" href={site.logo || 'data:,'} />
  <title>{title}</title>
  <meta name="description" content={description} />
  {#if canonical}<link rel="canonical" href={canonical} />{/if}
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Vut" />
  <meta property="og:title" content={title} />
  <meta property="og:description" content={description} />
  {#if canonical}<meta property="og:url" content={canonical} />{/if}
  <meta
    property="og:image"
    content={site.origin ? new URL('/og.png', site.origin).href : '/og.png'}
  />
  <meta property="og:image:alt" content="Vut — Build a brighter tomorrow" />
  <meta property="og:image:width" content="1200" />
  <meta property="og:image:height" content="630" />
  <meta name="twitter:card" content="summary_large_image" />
</svelte:head>

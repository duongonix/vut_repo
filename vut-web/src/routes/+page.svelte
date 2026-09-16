<script lang="ts">
  import Hero from '$lib/components/home/Hero.svelte';
  import Features from '$lib/components/home/Features.svelte';
  import WhyVut from '$lib/components/home/WhyVut.svelte';
  import Ecosystem from '$lib/components/home/Ecosystem.svelte';
  import Seo from '$lib/components/layout/Seo.svelte';
  let { data } = $props();
</script>

<Seo />
<main id="main-content">
  <Hero html={data.heroCode} />
  <Features />
  <WhyVut examples={data.examples} />
  <Ecosystem />
</main>

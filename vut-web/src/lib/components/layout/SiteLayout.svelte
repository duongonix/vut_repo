<script lang="ts">
  import '$lib/styles/global.css';
  import Header from '../navigation/Header.svelte';
  import Footer from './Footer.svelte';
  import SearchDialog from '../search/SearchDialog.svelte';
  import type { Snippet } from 'svelte';
  let { children }: { children: Snippet } = $props();
  let searchOpen = $state(false);

  function shortcuts(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      searchOpen = !searchOpen;
    }
  }
</script>

<svelte:window onkeydown={shortcuts} />
<a href="#main-content" class="skip-link">Skip to content</a>
<Header onsearch={() => (searchOpen = true)} />
{@render children()}
<Footer />
<SearchDialog bind:open={searchOpen} />

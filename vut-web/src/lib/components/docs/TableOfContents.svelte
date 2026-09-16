<script lang="ts">
  import type { Heading } from '$lib/config/docs';
  let { headings = [] }: { headings?: Heading[] } = $props();
  let active = $state('');
  $effect(() => {
    const entries = headings;
    active = entries[0]?.id || '';
    const observer = new IntersectionObserver(
      (changes) => {
        const visible = changes
          .filter((change) => change.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (visible[0]) active = visible[0].target.id;
      },
      { rootMargin: '-95px 0px -65% 0px', threshold: 0 }
    );
    entries.forEach((heading) => {
      const element = document.getElementById(heading.id);
      if (element) observer.observe(element);
    });
    return () => observer.disconnect();
  });
</script>

<nav aria-label="On this page">
  <h2>On this page</h2>
  {#each headings as heading}<a
      href={`#${heading.id}`}
      class:nested={heading.depth === 3}
      aria-current={active === heading.id ? 'location' : undefined}
      onclick={() => (active = heading.id)}>{heading.title}</a
    >{/each}
</nav>

<style>
  h2 {
    font-size: 12px;
    letter-spacing: 0;
    font-weight: 550;
    margin-bottom: 16px;
  }
  a {
    display: block;
    border-left: 1px solid var(--border);
    padding: 7px 0 7px 15px;
    color: var(--subtle);
    font-size: 12px;
    line-height: 1.5;
  }
  a.nested {
    padding-left: 26px;
  }
  a:hover {
    color: var(--text);
  }
  a[aria-current] {
    color: var(--primary);
    border-left-color: var(--primary);
  }
</style>

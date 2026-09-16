<script lang="ts">
  import { page } from '$app/state';
  import { ChevronDown, BookOpen } from '@lucide/svelte';
  import { docGroups } from '$lib/config/docs';
  import { site } from '$lib/config/site';
  import { goto } from '$app/navigation';
  import { tick, untrack } from 'svelte';
  let { onnavigate = () => {} }: { onnavigate?: () => void } = $props();
  let collapsed = $state<string[]>([]);
  const id = $props.id();
  let nav: HTMLElement;
  const current = $derived(page.url.pathname);
  $effect(() => {
    const section = docGroups.find((group) =>
      group.pages.some(([, slug]) => current === `/docs/${slug}/`)
    );
    untrack(() => {
      if (section) collapsed = collapsed.filter((title) => title !== section.title);
    });
    tick().then(() => {
      const active = nav?.querySelector<HTMLElement>('[aria-current="page"]');
      const scroller = nav?.closest<HTMLElement>('.sidebar, .drawer');
      if (!active || !scroller || !scroller.getClientRects().length) return;
      const item = active.getBoundingClientRect();
      const bounds = scroller.getBoundingClientRect();
      if (item.top < bounds.top || item.bottom > bounds.bottom)
        scroller.scrollTop += item.top - bounds.top - 60;
    });
  });
  function toggle(title: string) {
    collapsed = collapsed.includes(title)
      ? collapsed.filter((value) => value !== title)
      : [...collapsed, title];
  }
</script>

<nav bind:this={nav} aria-label="Documentation sidebar">
  <div class="version">
    <BookOpen size={17} /><label for={`${id}-version`}>Vut</label><select
      id={`${id}-version`}
      aria-label="Documentation version"
      onchange={(event) => goto(event.currentTarget.value)}
      >{#each site.versions as version}<option value={version.href}>{version.label}</option
        >{/each}</select
    >
  </div>
  {#each docGroups as group}
    {@const expanded = !collapsed.includes(group.title)}
    <div class="group">
      <button class="group-title" aria-expanded={expanded} onclick={() => toggle(group.title)}
        >{group.title}<ChevronDown size={13} class={expanded ? '' : 'collapsed'} /></button
      >
      {#if expanded}<ul>
          {#each group.pages as [title, slug]}<li>
              <a
                href={`/docs/${slug}/`}
                aria-current={current === `/docs/${slug}/` ? 'page' : undefined}
                onclick={onnavigate}>{title}</a
              >
            </li>{/each}
        </ul>{/if}
    </div>
  {/each}
</nav>

<style>
  nav {
    font-size: 13px;
  }
  .version {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 29px;
  }
  .version :global(svg) {
    color: var(--primary);
  }
  select {
    margin-left: auto;
    border: 0;
    color: var(--muted);
    background: var(--surface);
    border-radius: 4px;
    font-size: 12px;
    padding: 4px;
  }
  .group {
    margin-bottom: 24px;
  }
  .group-title {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    text-transform: uppercase;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.1em;
    background: none;
    color: var(--subtle);
    padding: 0 12px 9px;
    text-align: left;
  }
  .group-title :global(.collapsed) {
    transform: rotate(-90deg);
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  a {
    display: block;
    padding: 8px 12px;
    color: var(--muted);
    border-left: 2px solid transparent;
    border-radius: 0 6px 6px 0;
  }
  a:hover {
    background: var(--hover);
    color: var(--text);
  }
  a[aria-current] {
    border-left-color: var(--primary);
    background: #7868ff12;
    color: var(--primary);
  }
</style>

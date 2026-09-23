<script lang="ts">
  import { Search, GitFork, Menu, X, ArrowUpRight } from '@lucide/svelte';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import { navigation, site } from '$lib/config/site';
  import Brand from '../ui/Brand.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import Sidebar from '../docs/Sidebar.svelte';
  let { onsearch }: { onsearch: () => void } = $props();
  let drawer: HTMLDialogElement;
  afterNavigate(() => drawer?.close());
  function activeLink(label: string, href: string) {
    const path = page.url.pathname;
    if (label === 'Reference') return path.startsWith('/docs/reference/');
    if (label === 'Guide')
      return path.startsWith('/docs/getting-started/') && !path.endsWith('/introduction/');
    if (label === 'Docs')
      return (
        path.startsWith('/docs/') &&
        !path.startsWith('/docs/reference/') &&
        (!path.startsWith('/docs/getting-started/') || path.endsWith('/introduction/'))
      );
    return path === href;
  }
</script>

<header>
  <div class="header-inner container">
    <button
      class="icon-button mobile-menu"
      aria-label="Open navigation"
      onclick={() => drawer.showModal()}><Menu size={21} /></button
    >
    <Brand />
    <nav class="desktop-nav" aria-label="Main navigation">
      {#each navigation as link}
        <a
          href={link.href}
          class:active={activeLink(link.label, link.href)}
          aria-current={activeLink(link.label, link.href) ? 'location' : undefined}
          >{link.label}{#if link.label === 'Community'}<ArrowUpRight size={12} />{/if}</a
        >
      {/each}
    </nav>
    <div class="actions">
      <button class="search-button" onclick={onsearch} aria-label="Search documentation"
        ><Search size={16} /><span>Search docs...</span><kbd>⌘ K</kbd></button
      >
      {#if site.github}<a class="icon-button github" href={site.github} aria-label="Vut on GitHub"
          ><GitFork size={20} /></a
        >{/if}
      <ThemeToggle />
    </div>
  </div>
</header>

<dialog bind:this={drawer} class="drawer" aria-label="Navigation">
  <div class="drawer-top">
    <Brand /><button
      class="icon-button"
      aria-label="Close navigation"
      onclick={() => drawer.close()}><X size={20} /></button
    >
  </div>
  <nav aria-label="Mobile navigation">
    {#each navigation as link}<a href={link.href} onclick={() => drawer.close()}
        >{link.label}<ArrowUpRight size={14} /></a
      >{/each}
  </nav>
  <Sidebar onnavigate={() => drawer.close()} />
</dialog>

<style>
  header {
    position: sticky;
    top: 0;
    z-index: 30;
    background: color-mix(in srgb, var(--background) 87%, transparent);
    backdrop-filter: blur(14px);
    border-bottom: 1px solid var(--border);
  }
  .header-inner {
    height: 78px;
    display: flex;
    align-items: center;
    gap: 45px;
  }
  .desktop-nav {
    display: flex;
    align-items: center;
    gap: 29px;
    margin-left: 30px;
  }
  nav a {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    transition: color 0.15s;
  }
  nav a:hover,
  nav a.active {
    color: var(--primary);
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .search-button {
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--subtle);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 8px 10px;
    font-size: 12px;
  }
  .search-button:hover {
    border-color: var(--border-hover);
  }
  kbd {
    background: var(--hover);
    padding: 3px 5px;
    border-radius: 4px;
    margin-left: 26px;
    font-size: 10px;
  }
  .mobile-menu {
    display: none;
  }
  .drawer {
    margin: 0;
    width: min(360px, 90vw);
    max-height: 100dvh;
    height: 100dvh;
    padding: 24px;
    border: 0;
    border-right: 1px solid var(--border);
  }
  .drawer-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }
  .drawer nav {
    display: grid;
    gap: 4px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 24px;
  }
  .drawer nav a {
    padding: 12px 2px;
    justify-content: space-between;
  }
  @media (max-width: 1180px) {
    .header-inner {
      gap: 24px;
    }
    .desktop-nav {
      gap: 20px;
      margin-left: 10px;
    }
    kbd {
      margin-left: 8px;
    }
  }
  @media (max-width: 960px) {
    .desktop-nav {
      display: none;
    }
    .mobile-menu {
      display: inline-flex;
    }
    .header-inner {
      height: 66px;
      gap: 15px;
    }
  }
  @media (max-width: 640px) {
    .search-button {
      border: 0;
      background: none;
      padding: 9px;
    }
    .search-button span,
    kbd,
    .github {
      display: none;
    }
    .actions {
      gap: 2px;
    }
  }
</style>

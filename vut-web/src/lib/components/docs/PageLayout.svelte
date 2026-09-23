<script lang="ts">
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';
  import { ArrowLeft, ArrowRight, ChevronRight, ChevronDown } from '@lucide/svelte';
  import Sidebar from './Sidebar.svelte';
  import TableOfContents from './TableOfContents.svelte';
  import Seo from '../layout/Seo.svelte';
  import { docPages, type Frontmatter } from '$lib/config/docs';
  import { site } from '$lib/config/site';
  import { codeCopy } from '$lib/actions/code-copy';
  import { headingCopy } from '$lib/actions/heading-copy';
  import '$lib/styles/prose.css';
  let { fm, children }: { fm: Frontmatter; children: Snippet } = $props();
  const isDocs = $derived(fm.pageType === 'md');
  const index = $derived(docPages.findIndex((doc) => doc.href === page.url.pathname));
  const previous = $derived(index > 0 ? docPages[index - 1] : undefined);
  const next = $derived(index >= 0 ? docPages[index + 1] : undefined);
</script>

{#if isDocs}
  <Seo title={`${fm.title} — Vut Docs`} description={fm.description} />
  <div class="docs-shell container">
    <aside class="sidebar" aria-label="Documentation navigation"><Sidebar /></aside>
    <main id="main-content" class="article-column">
      <nav class="breadcrumb" aria-label="Breadcrumb">
        <a href="/docs/getting-started/introduction/">Docs</a><ChevronRight size={12} /><span
          >{fm.section || docPages[index]?.section || 'Documentation'}</span
        ><ChevronRight size={12} /><span>{fm.title}</span>
      </nav>
      <details class="mobile-toc">
        <summary>On this page <ChevronDown size={15} /></summary><TableOfContents
          headings={fm.headings}
        />
      </details>
      <article data-pagefind-body>
        <span class="sr-only" data-pagefind-meta="section"
          >{fm.section || docPages[index]?.section || 'Documentation'}</span
        >
        <h1 data-pagefind-meta="title">{fm.title}</h1>
        <p class="description">{fm.description}</p>
        <div class="prose" use:codeCopy use:headingCopy>{@render children()}</div>
      </article>
      {#if site.editBase}<a class="edit-link" href={`${site.editBase}${page.url.pathname}+page.md`}
          >Edit this page on GitHub <ArrowRight size={13} /></a
        >{/if}
      <nav class="pagination" aria-label="Documentation pagination">
        {#if previous}<a href={previous.href}
            ><span><ArrowLeft size={13} /> Previous</span><strong>{previous.title}</strong></a
          >{:else}<div></div>{/if}
        {#if next}<a class="next" href={next.href}
            ><span>Next <ArrowRight size={13} /></span><strong>{next.title}</strong></a
          >{/if}
      </nav>
    </main>
    <aside class="toc" aria-label="Article contents">
      <TableOfContents headings={fm.headings} />
    </aside>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .docs-shell {
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr) 180px;
    gap: 45px;
    align-items: start;
    padding-top: 40px;
    padding-bottom: 65px;
  }
  .sidebar,
  .toc {
    position: sticky;
    top: 110px;
    max-height: calc(100dvh - 135px);
    overflow-y: auto;
    scrollbar-width: thin;
  }
  .sidebar {
    padding-right: 10px;
  }
  .toc {
    padding-top: 9px;
  }
  .article-column {
    min-width: 0;
    max-width: 850px;
  }
  .breadcrumb {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
    font-size: 11px;
    color: var(--subtle);
    margin: 7px 0 29px;
  }
  .breadcrumb a:hover {
    color: var(--primary);
  }
  h1 {
    font-size: 42px;
    font-weight: 620;
    line-height: 1.15;
    margin-bottom: 18px;
  }
  .description {
    font-size: 17px;
    line-height: 1.7;
    padding-bottom: 27px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 26px;
  }
  .pagination {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    padding-top: 40px;
    margin-top: 35px;
    border-top: 1px solid var(--border);
  }
  .pagination a {
    border: 1px solid var(--border);
    padding: 17px 20px;
    border-radius: 10px;
    background: var(--surface);
  }
  .pagination a:hover {
    border-color: var(--primary);
  }
  .pagination span {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--subtle);
    font-size: 11px;
    margin-bottom: 10px;
  }
  .pagination strong {
    font-size: 13px;
    color: var(--primary);
    font-weight: 500;
  }
  .next {
    text-align: right;
  }
  .next span {
    justify-content: end;
  }
  .edit-link {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    font-size: 12px;
    color: var(--subtle);
    margin-top: 32px;
  }
  .mobile-toc {
    display: none;
  }
  @media (min-width: 1500px) {
    .docs-shell {
      grid-template-columns: 250px minmax(0, 1fr) 200px;
    }
  }
  @media (max-width: 1180px) {
    .docs-shell {
      grid-template-columns: 215px minmax(0, 1fr);
      gap: 35px;
    }
    .toc {
      display: none;
    }
    .mobile-toc {
      display: block;
      margin-bottom: 25px;
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 12px 16px;
    }
    summary {
      display: flex;
      align-items: center;
      justify-content: space-between;
      cursor: pointer;
      font-size: 12px;
      color: var(--muted);
    }
    details[open] summary {
      margin-bottom: 20px;
    }
  }
  @media (max-width: 960px) {
    .docs-shell {
      grid-template-columns: minmax(0, 1fr);
      max-width: 800px;
      padding-top: 24px;
    }
    .sidebar {
      display: none;
    }
  }
  @media (max-width: 480px) {
    h1 {
      font-size: 35px;
    }
    .description {
      font-size: 16px;
    }
    .pagination a {
      padding: 14px;
    }
  }
</style>

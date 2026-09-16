import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { sveltepress } from '@sveltepress/vite';
import { markdownHighlighter } from './src/lib/server/highlight.ts';
import { docsHeadings } from './src/lib/server/headings.ts';

export default defineConfig({
  plugins: [
    sveltepress({
      siteConfig: { title: 'Vut', description: 'A modern, expressive programming language.' },
      theme: {
        name: 'vut',
        globalLayout: resolve('src/lib/components/layout/SiteLayout.svelte').replaceAll('\\', '/'),
        pageLayout: resolve('src/lib/components/docs/PageLayout.svelte').replaceAll('\\', '/'),
        vitePlugins: [],
        highlighter: markdownHighlighter,
        rehypePlugins: [docsHeadings]
      },
      pagefind: { rootSelector: '[data-pagefind-body]', forceLanguage: 'en' }
    })
  ]
});

import { cp } from 'node:fs/promises';

// SveltePress indexes adapter-static's build/ after prerendering. Vite preview
// serves SvelteKit's client output instead, so expose the same generated index.
await cp(
  new URL('../build/pagefind/', import.meta.url),
  new URL('../.svelte-kit/output/client/pagefind/', import.meta.url),
  { recursive: true }
);
console.log('Pagefind index ready for production preview');

import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  extensions: ['.svelte', '.md'],
  preprocess: vitePreprocess(),
  kit: { adapter: adapter({ pages: 'build', assets: 'build', fallback: '404.html' }) }
};

import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

// Static output only: every route is built ahead of time (build-web-experience W1).
export default defineConfig({
  site: 'https://geoprims.com',
  output: 'static',
  trailingSlash: 'always',
  integrations: [svelte()],
  build: { format: 'directory', inlineStylesheets: 'never' },
  vite: {
    server: { fs: { allow: ['../..'] } },
    worker: { format: 'es' },
  },
});

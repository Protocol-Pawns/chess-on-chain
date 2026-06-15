import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  site: 'https://protocol-pawns.com',
  output: 'static',
  vite: {
    plugins: [tailwindcss()]
  },
  build: {
    format: 'file'
  },
  compressHTML: true,
  prefetch: {
    prefetchAll: true
  }
});

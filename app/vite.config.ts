import { sveltekit } from '@sveltejs/kit/vite';
import UnoCSS from 'unocss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [UnoCSS(), sveltekit()],
	ssr: {
		external: ['util', 'is-my-json-valid', 'generate-function', 'jsonpointer']
	}
});

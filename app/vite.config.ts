import { sveltekit } from '@sveltejs/kit/vite';
import UnoCSS from 'unocss/vite';
import { defineConfig } from 'vite';
import { nodePolyfills } from 'vite-plugin-node-polyfills';

export default defineConfig({
	plugins: [UnoCSS(), sveltekit(), nodePolyfills({ include: ['buffer'] })],
	ssr: {
		external: ['util', 'is-my-json-valid', 'generate-function', 'jsonpointer']
	}
});

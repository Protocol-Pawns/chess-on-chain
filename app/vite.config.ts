import { sveltekit } from '@sveltejs/kit/vite';
import { execSync } from 'child_process';
import UnoCSS from 'unocss/vite';
import { defineConfig } from 'vite';

function getVersion() {
  try {
    return execSync('git rev-parse --short HEAD', { encoding: 'utf-8' }).trim();
  } catch {
    return new Date().toISOString().replace(/[:.]/g, '-');
  }
}

const version = getVersion();

function versionPlugin() {
  return {
    name: 'version-plugin',
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    generateBundle(this: any) {
      this.emitFile({
        type: 'asset',
        fileName: 'version.json',
        source: JSON.stringify({ version }) + '\n'
      });
    }
  };
}

export default defineConfig({
  plugins: [UnoCSS(), sveltekit(), versionPlugin()],
  ssr: {
    external: ['util', 'is-my-json-valid', 'generate-function', 'jsonpointer']
  },
  define: {
    __APP_VERSION__: JSON.stringify(version)
  }
});

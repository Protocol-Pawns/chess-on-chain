import {
  copyFileSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = resolve(__dirname, '../src');
const generatedDir = resolve(srcDir, 'generated');

function findWasm() {
  const local = resolve(__dirname, '../node_modules/@resvg/resvg-wasm/index_bg.wasm');
  const root = resolve(__dirname, '../../node_modules/@resvg/resvg-wasm/index_bg.wasm');
  for (const candidate of [local, root]) {
    try {
      copyFileSync(candidate, candidate); // cheap existence check
      return candidate;
    } catch {
      // try next
    }
  }
  throw new Error('Could not find @resvg/resvg-wasm/index_bg.wasm');
}

mkdirSync(srcDir, { recursive: true });
mkdirSync(generatedDir, { recursive: true });

// Copy resvg WASM into src so wrangler bundles it as a CompiledWasm module.
const wasmSource = findWasm();
const wasmDest = resolve(srcDir, 'resvg.wasm');
copyFileSync(wasmSource, wasmDest);
console.log('Copied resvg.wasm to', wasmDest);

// Bundle chess piece PNGs as base64 strings.
const piecesDir = resolve(__dirname, '../../app/static/pieces-png');
const pieceFiles = readdirSync(piecesDir).filter(f => f.endsWith('.png'));
const pieces = {};
for (const file of pieceFiles) {
  const key = file.replace('.png', ''); // e.g. "wP"
  const b64 = readFileSync(resolve(piecesDir, file)).toString('base64');
  pieces[key] = b64;
}
writeFileSync(
  resolve(generatedDir, 'pieces.ts'),
  `export const PIECES: Record<string, string> = ${JSON.stringify(pieces, null, 2)};\n`
);
console.log('Generated generated/pieces.ts with', pieceFiles.length, 'pieces');

// Bundle a font for SVG text rendering.
const fontPath = resolve(__dirname, '../assets/DejaVuSans.ttf');
const fontB64 = readFileSync(fontPath).toString('base64');
writeFileSync(
  resolve(generatedDir, 'font.ts'),
  `export const FONT_BASE64 = ${JSON.stringify(fontB64)};\n`
);
console.log('Generated generated/font.ts');

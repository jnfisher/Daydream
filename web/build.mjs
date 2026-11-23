// Bundles the UI into the server's sources: crates/daydream-server/src/app.js and app.css.
// The output is committed, so building the server needs no node; CI checks it is current.
import { build } from 'esbuild';
import { copyFileSync } from 'node:fs';

const out = new URL('../crates/daydream-server/src/', import.meta.url);
await build({
  entryPoints: [new URL('src/main.ts', import.meta.url).pathname],
  bundle: true,
  format: 'esm',
  target: 'es2022',
  minify: false,
  legalComments: 'none',
  banner: { js: '// Built from web/src by `npm run build` in web/. Do not edit.' },
  outfile: new URL('app.js', out).pathname,
});
copyFileSync(new URL('app.css', import.meta.url), new URL('app.css', out));

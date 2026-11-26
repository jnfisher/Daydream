// Bundles the UI into the server's sources (crates/daydream-server/src): app.js and app.css,
// the two fonts in fonts/, and notices.txt, the licenses of every package the bundle holds.
// The output is committed, so building the server needs no node; CI checks it is current
// and within the size budget (budget.mjs).
import { build } from 'esbuild';
import svelte from 'esbuild-svelte';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const here = (p) => new URL(p, import.meta.url).pathname;
const out = here('../crates/daydream-server/src/');

/** The fonts the CSS asks for at /fonts/NAME, from their Fontsource packages. */
const FONTS = [
  ['nunito.woff2', '@fontsource-variable/nunito', 'files/nunito-latin-wght-normal.woff2'],
  ['fredoka.woff2', '@fontsource-variable/fredoka', 'files/fredoka-latin-wght-normal.woff2'],
];

const result = await build({
  entryPoints: [here('src/main.ts')],
  bundle: true,
  format: 'esm',
  target: 'es2022',
  minify: true,
  legalComments: 'none',
  conditions: ['svelte', 'browser', 'production'],
  mainFields: ['svelte', 'browser', 'module', 'main'],
  plugins: [svelte({ compilerOptions: { css: 'external', dev: false } })],
  // served by the server itself, not bundled
  external: ['/fonts/*'],
  banner: {
    js: '// Built from web/src by `npm run build` in web/. Do not edit. Licenses: /notices.txt',
    css: '/* Built from web/src by `npm run build` in web/. Do not edit. Licenses: /notices.txt */',
  },
  outfile: join(out, 'app.js'),
  metafile: true,
  logLevel: 'warning',
});

mkdirSync(join(out, 'fonts'), { recursive: true });
for (const [name, pkg, file] of FONTS) copyFileSync(here(`node_modules/${pkg}/${file}`), join(out, 'fonts', name));

// Every package with code in the bundle, plus the fonts: name, version, license and text.
const packages = new Set(FONTS.map(([, pkg]) => pkg));
for (const input of Object.keys(result.metafile.inputs)) {
  const m = /(?:^|\/)node_modules\/((?:@[^/]+\/)?[^/]+)\//.exec(input);
  if (m) packages.add(m[1]);
}
const notices = [...packages].sort().map((pkg) => {
  const dir = here(`node_modules/${pkg}/`);
  const meta = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  const file = ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'license', 'license.md', 'LICENCE', 'OFL.txt']
    .map((f) => join(dir, f))
    .find(existsSync);
  if (!file) throw new Error(`${pkg} has no license file to ship`);
  const text = readFileSync(file, 'utf8').replace(/\r\n/g, '\n').trim();
  return `== ${meta.name} ${meta.version} (${meta.license}) ==\n\n${text}\n`;
});
writeFileSync(
  join(out, 'notices.txt'),
  "Daydream's web UI is built from these packages, whose licenses follow.\n\n" + notices.join('\n'),
);

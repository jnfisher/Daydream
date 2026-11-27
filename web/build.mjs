// Bundles the UI into the server's sources (crates/daydream-server/src): app.js and app.css,
// the code split from it in chunks/ (listed for the server in chunks.rs), the
// two fonts in fonts/, and notices.txt, the licenses of every package the bundle holds (and
// of the spelling check's word list, which the server holds).
// The output is committed, so building the server needs no node; CI checks it is current
// and within the size budget (budget.mjs).
import { build } from 'esbuild';
import svelte from 'esbuild-svelte';
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const here = (p) => new URL(p, import.meta.url).pathname;
const out = here('../crates/daydream-server/src/');

/** The fonts the CSS asks for at /fonts/NAME, from their Fontsource packages. */
const FONTS = [
  ['nunito.woff2', '@fontsource-variable/nunito', 'files/nunito-latin-wght-normal.woff2'],
  ['fredoka.woff2', '@fontsource-variable/fredoka', 'files/fredoka-latin-wght-normal.woff2'],
];

// chunks are named by their contents: the last build's go
rmSync(join(out, 'chunks'), { recursive: true, force: true });

const result = await build({
  entryPoints: { app: here('src/main.ts') },
  bundle: true,
  format: 'esm',
  // a place's code (the map) loads when the place is first opened, not with the first page
  splitting: true,
  chunkNames: 'chunks/[name]-[hash]',
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
  outdir: out,
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
// the spelling check's dictionary is SCOWL's (crates/daydream-nmr/src/words, tools/words/build.mjs)
const scowl = readFileSync(here('../crates/daydream-nmr/src/words/COPYRIGHT.txt'), 'utf8').replace(/\r\n/g, '\n').trim();
writeFileSync(
  join(out, 'notices.txt'),
  "Daydream's web UI is built from these packages, whose licenses follow. After them comes the notice of SCOWL, the word lists its spelling check reads.\n\n" +
    notices.join('\n') +
    `\n== SCOWL (Spell Checker Oriented Word Lists), as wordlist-english 1.2.1 ships it ==\n\n${scowl}\n`,
);

// A chunk is named by a hash of its code and of the paths of the files in it. A component's
// external styles come in under their absolute path, so a chunk holding them would be named for
// where the checkout lives, and CI would build it under another name.
for (const [file, output] of Object.entries(result.metafile.outputs)) {
  const styles = Object.keys(output.inputs).find((input) => input.startsWith('fakecss:'));
  if (styles && file.endsWith('.js') && file.includes('/chunks/')) {
    const component = relative(here('.'), styles.replace(/^fakecss:/, '').replace(/\.esbuild-svelte-fake-css$/, '.svelte'));
    throw new Error(`${file.slice(file.lastIndexOf('/') + 1)} holds the styles of ${component}: give it <svelte:options css="injected" />`);
  }
}

// the server serves each chunk by name (crates/daydream-server/src/chunks.rs)
const appCss = readFileSync(join(out, 'app.css'), 'utf8');
for (const f of existsSync(join(out, 'chunks')) ? readdirSync(join(out, 'chunks')) : []) {
  if (f.endsWith('.js')) continue;
  // nothing would load a chunk's styles: they must be app.css's already (a package's, shared
  // with the first page), and a component loaded later keeps its own in its code
  const text = readFileSync(join(out, 'chunks', f), 'utf8').replace(/^\/\*.*?\*\/\n/, '').trim();
  if (!appCss.includes(text)) throw new Error(`${f}: give the component that styles it <svelte:options css="injected" />`);
  rmSync(join(out, 'chunks', f));
}
const chunks = existsSync(join(out, 'chunks')) ? readdirSync(join(out, 'chunks')).sort() : [];
writeFileSync(
  join(out, 'chunks.rs'),
  `//! The UI's code split from app.js by \`npm run build\` in web/ (do not edit): each place's own code,
//! loaded when the place is first opened, and the code app.js shares with them. A chunk's name
//! changes with its contents, so a browser may keep it for good.

/// A chunk's code, by its file name.
pub(crate) fn get(name: &str) -> Option<&'static str> {
    Some(match name {
${chunks.map((f) => `        ${JSON.stringify(f)} => include_str!(${JSON.stringify(`chunks/${f}`)}),\n`).join('')}        _ => return None,
    })
}
`,
);

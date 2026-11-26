// The UI's size budget, gzipped: what a browser downloads the first time it opens Daydream
// (app.js and the chunks it imports, and app.css), and each place's code that loads when the
// place is first opened. CI runs this after `npm run build`; raise a limit only on purpose.
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';

const BUDGET = { first: 175, css: 25, place: 40 };

const dir = new URL('../crates/daydream-server/src/', import.meta.url);
const read = (f) => readFileSync(new URL(f, dir));
const gz = (bytes) => gzipSync(bytes, { level: 9 }).length / 1024;
const kb = (n) => `${n.toFixed(1)} KB`;

let over = false;
function check(what, size, raw, limit) {
  const line = `${what}: ${kb(size)} gzipped (${(raw / 1024).toFixed(0)} KB), budget ${limit} KB`;
  if (size > limit) {
    over = true;
    console.error(`::error::${line}`);
  } else console.log(line);
}

// the first page: app.js and what it imports at once (not what it imports when a place opens)
const app = read('app.js');
const first = [...app.toString().matchAll(/(?:^|[;\s}])import\s*(?:[\w$*{},\s]*from\s*)?["']\.\/(chunks\/[^"']+)["']/g)].map((m) => m[1]);
let size = gz(app);
let raw = app.length;
for (const f of first) {
  const b = read(f);
  size += gz(b);
  raw += b.length;
}
check(`app.js${first.length ? ` and ${first.length} chunk${first.length > 1 ? 's' : ''} it imports` : ''}`, size, raw, BUDGET.first);
const css = read('app.css');
check('app.css', gz(css), css.length, BUDGET.css);

// each place's own code
const chunks = existsSync(new URL('chunks/', dir)) ? readdirSync(new URL('chunks/', dir)) : [];
for (const f of chunks.sort()) {
  if (first.includes(`chunks/${f}`)) continue;
  const b = read(`chunks/${f}`);
  check(`chunks/${f}, loaded when first used`, gz(b), b.length, BUDGET.place);
}
process.exit(over ? 1 : 0);

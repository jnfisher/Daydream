// The UI's size budget, gzipped: what a browser downloads the first time it opens Daydream.
// CI runs this after `npm run build`; raise a limit only on purpose.
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';

const BUDGET = { 'app.js': 175, 'app.css': 25 };

let over = false;
for (const [file, kb] of Object.entries(BUDGET)) {
  const bytes = readFileSync(new URL(`../crates/daydream-server/src/${file}`, import.meta.url));
  const gz = gzipSync(bytes, { level: 9 }).length / 1024;
  const line = `${file}: ${gz.toFixed(1)} KB gzipped (${(bytes.length / 1024).toFixed(0)} KB), budget ${kb} KB`;
  if (gz > kb) {
    over = true;
    console.error(`::error::${line}`);
  } else console.log(line);
}
process.exit(over ? 1 : 0);

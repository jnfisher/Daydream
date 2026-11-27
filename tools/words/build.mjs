// Builds crates/daydream-nmr/src/words/, the spelling check's dictionary: words.deflate (raw
// deflate of the words, one lower-case word to a line, sorted) and COPYRIGHT.txt, from the
// SCOWL word lists (http://wordlist.aspell.net/) as the npm package wordlist-english 1.2.1
// ships them: every dialect's words (American, British, Canadian, Australian and the shared
// ones) of sizes 10 to 60, the common and less common words but not the rarest.
//
// Run from the repository's root: node tools/words/build.mjs (it fetches the package with npm).

import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { deflateRawSync } from 'node:zlib';

const PACKAGE = 'wordlist-english@1.2.1';
const LARGEST = 60;
const out = new URL('../../crates/daydream-nmr/src/words/', import.meta.url).pathname;

const tmp = mkdtempSync(join(tmpdir(), 'daydream-words-'));
try {
  const tgz = execFileSync('npm', ['pack', PACKAGE, '--silent'], { cwd: tmp, encoding: 'utf8' }).trim().split('\n').pop();
  execFileSync('tar', ['-xzf', tgz], { cwd: tmp });
  const dir = join(tmp, 'package');
  const words = new Set();
  for (const f of readdirSync(dir)) {
    const m = /^[a-z]+-words-(\d+)\.json$/.exec(f);
    if (!m || Number(m[1]) > LARGEST) continue;
    for (const w of JSON.parse(readFileSync(join(dir, f), 'utf8'))) words.add(w.toLowerCase());
  }
  const list = [...words].sort().join('\n');
  mkdirSync(out, { recursive: true });
  writeFileSync(join(out, 'words.deflate'), deflateRawSync(Buffer.from(list), { level: 9 }));
  writeFileSync(join(out, 'COPYRIGHT.txt'), readFileSync(join(dir, 'Copyright'), 'utf8').replace(/\r\n/g, '\n'));
  console.log(`${words.size} words`);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}

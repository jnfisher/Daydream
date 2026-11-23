// NMR's Paste Character (frmUser's PasteCharacter): a capture of the game's "stat" output,
// or MMUD Explorer's "Copy Only Stats", read into the user editor's fields.

/** NMR's `GetNextNumbers` then VB's `Val`: from `at`, the run of digits and spaces (a `*`
 *  restarts it), its spaces dropped. Empty when there is none. */
export function nextNumber(text: string, at: number): string {
  let start = at;
  let y = at;
  for (; y < text.length; y++) {
    const c = text[y]!;
    if (c === '*') start = y + 1;
    else if (!/[0-9 ]/.test(c)) break;
  }
  if (y <= start) return '';
  const digits = text.slice(start, y).replace(/ /g, '');
  return digits === '' ? '0' : String(Number(digits));
}

/** The text after `label` up to `next` (when it comes within 15 characters, on the same
 *  line) or the line's end, trimmed: NMR's race and class names. */
function nameAfter(text: string, label: string, next: string): string {
  const i = text.indexOf(label);
  if (i < 0) return '';
  const x = i + label.length;
  let y = text.indexOf(next, x);
  if (y > x + 15 || (y >= 0 && text.slice(x, y).trim().includes('\n'))) y = -1;
  if (y < 0) y = text.indexOf('\r', x);
  if (y < 0) y = text.indexOf('\n', x);
  return y > x ? text.slice(x, y).trim() : '';
}

export interface PastedStats {
  /** Column values, as NMR fills its fields. */
  values: Record<string, string>;
  race: string;
  cls: string;
}

/** Reads a stat capture as NMR does: each label's first number, where it is at least 1
 *  (Exp at least 0); each stat sets both its base and current value. */
export function parseStats(text: string): PastedStats {
  const values: Record<string, string> = {};
  const take = (label: string, columns: string[], min = 1) => {
    const i = text.indexOf(label);
    if (i < 0) return;
    const n = nextNumber(text, i + label.length);
    if (n === '' || Number(n) < min) return;
    for (const c of columns) values[c] = n;
  };
  for (const [label, stat] of [
    ['Intellect: ', 'INT'],
    ['Willpower: ', 'WIL'],
    ['Strength: ', 'STR'],
    ['Health: ', 'HEA'],
    ['Agility: ', 'AGL'],
    ['Charm: ', 'CHM'],
  ] as const) {
    take(label, [`Base ${stat}`, stat]);
  }
  const exp = text.indexOf('Exp: ');
  if (exp >= 0) {
    const n = nextNumber(text, exp + 5);
    if (n !== '') {
      // the record holds experience as billions and the rest
      const e = Number(n);
      values['Exp Billions'] = String(Math.floor(e / 1e9));
      values['Exp'] = String(e % 1e9);
    }
  }
  take('Level: ', ['Level']);
  take('Hits: ', ['HP']);
  take('Mana: ', ['Mana']);
  take('Kai: ', ['Mana']);
  take('Lives/CP:', ['Lives']);
  // CP: the number after the slash that follows the first "CP:"
  const cp = text.indexOf('CP:');
  if (cp >= 0) {
    const slash = text.indexOf('/', cp + 3);
    if (slash >= 0 && slash + 1 < text.length) {
      const n = nextNumber(text, slash + 1);
      if (n !== '') values['CP'] = n;
    }
  }
  return { values, race: nameAfter(text, 'Race: ', 'Exp:'), cls: nameAfter(text, 'Class: ', 'Level:') };
}

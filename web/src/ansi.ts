// MajorMUD text as a terminal shows it: ANSI colour escapes rendered, other escapes dropped.
// In the editors' inputs ESC shows as ␛ (U+241B), which can be typed or pasted; `toWire`
// turns it back into ESC before a save.

export const ESC_SHOWN = '␛';

export const toShown = (s: string) => s.replaceAll('\x1b', ESC_SHOWN);
export const toWire = (s: string) => s.replaceAll(ESC_SHOWN, '\x1b');

// the 16 colours of a DOS terminal: normal, then bright (bold)
const COLOURS = [
  '#000000', '#aa0000', '#00aa00', '#aa5500', '#0000aa', '#aa00aa', '#00aaaa', '#aaaaaa',
  '#555555', '#ff5555', '#55ff55', '#ffff55', '#5555ff', '#ff55ff', '#55ffff', '#ffffff',
];

interface Pen {
  fg: number;
  bg: number;
  bold: boolean;
}

/** Text with ANSI escapes (ESC or ␛) as spans in a terminal's colours. */
export function renderAnsi(text: string): DocumentFragment {
  const out = document.createDocumentFragment();
  const pen: Pen = { fg: 7, bg: 0, bold: false };
  const s = toWire(text);
  let run = '';
  const flush = () => {
    if (!run) return;
    const span = document.createElement('span');
    span.textContent = run;
    span.style.color = COLOURS[pen.fg + (pen.bold ? 8 : 0)] ?? COLOURS[7]!;
    if (pen.bg) span.style.backgroundColor = COLOURS[pen.bg] ?? '';
    out.append(span);
    run = '';
  };
  for (let i = 0; i < s.length; i++) {
    const c = s[i]!;
    if (c !== '\x1b') {
      if (c !== '\r') run += c;
      continue;
    }
    const m = /^\[([0-9;]*)([A-Za-z])/.exec(s.slice(i + 1));
    if (!m) continue;
    i += m[0].length;
    if (m[2] !== 'm') continue; // cursor and screen controls: no colour
    flush();
    const codes = (m[1] || '0').split(';').map((x) => Number(x || 0));
    for (const code of codes) {
      if (code === 0) Object.assign(pen, { fg: 7, bg: 0, bold: false });
      else if (code === 1) pen.bold = true;
      else if (code === 22) pen.bold = false;
      else if (code >= 30 && code <= 37) pen.fg = code - 30;
      else if (code >= 40 && code <= 47) pen.bg = code - 40;
    }
  }
  flush();
  return out;
}

/** A text's first non-blank line without escapes, at most 60 characters (as row lists show it).
 * A NUL ends the text: what follows one is an older value's leftovers. */
export function titleOf(text: string): string {
  const line = toWire(text).split('\0')[0]!.split(/[\r\n]/).find((l) => l.trim()) ?? '';
  const clean = line.replace(/\x1b\[[0-9;]*[A-Za-z]?/g, '').trim();
  return clean.length > 60 ? `${clean.slice(0, 60)}...` : clean;
}

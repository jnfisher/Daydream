// MajorMUD text as a terminal shows it: ANSI colour escapes rendered, a step of the cursor
// forward as the spaces it leaves (some blocks space their words so), other escapes dropped.
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

/** How many columns a cursor-forward escape's number moves, as a terminal reads it. */
const forward = (n: string) => ' '.repeat(Math.min(Math.max(Number(n) || 1, 1), 80));

/** Text without its escapes, a step of the cursor forward as its spaces. */
export function plainOf(text: string): string {
  return toWire(text)
    .replace(/\x1b\[([0-9]*)C/g, (_, n: string) => forward(n))
    .replace(/\x1b\[[0-9;]*[A-Za-z]?/g, '');
}

/** How a terminal draws text: its colour, its background, and whether it is bright. */
export interface Pen {
  fg: number;
  bg: number;
  bold: boolean;
}

/** A terminal's pen when nothing has set it. */
export const PEN: Pen = { fg: 7, bg: 0, bold: false };

/** Text with ANSI escapes (ESC or ␛) as runs of text, each with the pen it is drawn with,
 * from the pen `from` on; and the pen it leaves the terminal with. */
export function runsOf(text: string, from: Pen = PEN): { runs: [string, Pen][]; pen: Pen } {
  const runs: [string, Pen][] = [];
  const pen: Pen = { ...from };
  const s = toWire(text);
  let run = '';
  const flush = () => {
    if (run) runs.push([run, { ...pen }]);
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
    if (m[2] === 'C' && !m[1]!.includes(';')) {
      // the cursor forward: the columns it skips show as spaces
      run += forward(m[1]!);
      continue;
    }
    if (m[2] !== 'm') continue; // other cursor and screen controls: nothing shown
    flush();
    const codes = (m[1] || '0').split(';').map((x) => Number(x || 0));
    for (const code of codes) {
      if (code === 0) Object.assign(pen, PEN);
      else if (code === 1) pen.bold = true;
      else if (code === 22) pen.bold = false;
      else if (code >= 30 && code <= 37) pen.fg = code - 30;
      else if (code >= 40 && code <= 47) pen.bg = code - 40;
    }
  }
  flush();
  return { runs, pen };
}

/** Text with ANSI escapes (ESC or ␛) as spans in a terminal's colours, from the pen `from`
 * on (a terminal's colours carry on from what it showed before). */
export function renderAnsi(text: string, from: Pen = PEN): DocumentFragment {
  const out = document.createDocumentFragment();
  for (const [run, pen] of runsOf(text, from).runs) {
    const span = document.createElement('span');
    span.textContent = run;
    span.style.color = COLOURS[pen.fg + (pen.bold ? 8 : 0)] ?? COLOURS[7]!;
    if (pen.bg) span.style.backgroundColor = COLOURS[pen.bg] ?? '';
    out.append(span);
  }
  return out;
}

/** A text's first non-blank line without escapes, at most 60 characters (as row lists show it).
 * A NUL ends the text: what follows one is an older value's leftovers. */
export function titleOf(text: string): string {
  const line = toWire(text).split('\0')[0]!.split(/[\r\n]/).find((l) => l.trim()) ?? '';
  const clean = plainOf(line).trim();
  return clean.length > 60 ? `${clean.slice(0, 60)}...` : clean;
}

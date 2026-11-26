// This browser's choices: the theme and the little sounds. They live in localStorage, which
// can be missing (a private window), so every read and write is allowed to fail.

export type Theme = 'auto' | 'daydream' | 'starlight';

export const THEMES: { id: Theme; name: string; sub: string; swatches: string[] }[] = [
  { id: 'auto', name: 'Match my system', sub: 'Daydream by day, Starlight by night', swatches: ['#f3f5ff', '#ff7eb6', '#13112a', '#b8a5ff'] },
  { id: 'daydream', name: 'Daydream', sub: 'Pastel paper under a morning sky', swatches: ['#f3f5ff', '#ff7eb6', '#6cc2ff', '#ffd25e'] },
  { id: 'starlight', name: 'Starlight', sub: 'Indigo night, glowing pastels', swatches: ['#13112a', '#ff8dc4', '#7fcbff', '#b8a5ff'] },
];

export function read(key: string, fallback: string): string {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

export function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // a private window: the choice lasts until the page closes
  }
}

const stored = read('dd-theme', 'auto');

export const prefs = $state({
  theme: (THEMES.some((t) => t.id === stored) ? stored : 'auto') as Theme,
  sound: read('dd-sound', 'off') === 'on',
});

/** Applies a theme now and remembers it (index.html applies it before the first paint). */
export function setTheme(theme: Theme): void {
  prefs.theme = theme;
  write('dd-theme', theme);
  if (theme === 'auto') delete document.documentElement.dataset.dd;
  else document.documentElement.dataset.dd = theme;
}

export function setSound(on: boolean): void {
  prefs.sound = on;
  write('dd-sound', on ? 'on' : 'off');
  if (on) sfx('pop');
}

let audio: AudioContext | null = null;

const NOTES: Record<string, [number, number, number][]> = {
  pop: [[660, 0, 0.07], [990, 0.05, 0.08]],
  toggle: [[880, 0, 0.05]],
  undo: [[520, 0, 0.06], [390, 0.05, 0.08]],
  nope: [[180, 0, 0.12], [150, 0.1, 0.14]],
  tada: [[523, 0, 0.09], [659, 0.08, 0.09], [784, 0.16, 0.1], [1047, 0.24, 0.2]],
};

/** A soft synthesized sound, when the little sounds are on. */
export function sfx(kind: keyof typeof NOTES): void {
  if (!prefs.sound) return;
  try {
    audio ??= new AudioContext();
    const t0 = audio.currentTime;
    for (const [freq, at, dur] of NOTES[kind] ?? []) {
      const o = audio.createOscillator();
      const g = audio.createGain();
      o.type = kind === 'nope' ? 'square' : 'triangle';
      o.frequency.setValueAtTime(freq, t0 + at);
      g.gain.setValueAtTime(0.0001, t0 + at);
      g.gain.exponentialRampToValueAtTime(kind === 'nope' ? 0.04 : 0.08, t0 + at + 0.01);
      g.gain.exponentialRampToValueAtTime(0.0001, t0 + at + dur);
      o.connect(g).connect(audio.destination);
      o.start(t0 + at);
      o.stop(t0 + at + dur + 0.02);
    }
  } catch {
    // no audio here
  }
}

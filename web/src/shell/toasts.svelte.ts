// Short notes at the bottom of the screen: what just happened, with an action to take it
// back where there is one.

export interface Toast {
  id: number;
  text: string;
  kind: 'info' | 'error';
  action?: string;
  onAction?: () => void;
}

export const toasts: Toast[] = $state([]);

let next = 1;

/** Shows a toast; returns its id, for `dismiss`. */
export function toast(
  text: string,
  opts: { kind?: Toast['kind']; action?: string; onAction?: () => void; ms?: number } = {},
): number {
  const t: Toast = { id: next++, text, kind: opts.kind ?? 'info', action: opts.action, onAction: opts.onAction };
  toasts.push(t);
  // a few at most: the oldest go first
  while (toasts.length > 3) toasts.shift();
  setTimeout(() => dismiss(t.id), opts.ms ?? (t.kind === 'error' ? 8000 : t.action ? 6000 : 3500));
  return t.id;
}

export function dismiss(id: number): void {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

/** What went wrong, for a person. */
export const messageOf = (e: unknown) => (e instanceof Error ? e.message : String(e));

// Times as people say them, from the server's seconds since 1970.

const clock = (d: Date) => d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });

/** "just now", "5 min ago", "2 h ago", "yesterday 9:10 PM", "Nov 2". */
export function ago(seconds: number, now = Date.now()): string {
  const d = new Date(seconds * 1000);
  const s = Math.max(0, (now - d.getTime()) / 1000);
  if (s < 45) return 'just now';
  if (s < 60 * 60) return `${Math.round(s / 60)} min ago`;
  if (s < 6 * 60 * 60) return `${Math.round(s / 3600)} h ago`;
  const today = new Date(now);
  if (d.toDateString() === today.toDateString()) return `today ${clock(d)}`;
  const yesterday = new Date(now - 86400 * 1000);
  if (d.toDateString() === yesterday.toDateString()) return `yesterday ${clock(d)}`;
  return d.toLocaleDateString([], { month: 'short', day: 'numeric', year: d.getFullYear() === today.getFullYear() ? undefined : 'numeric' });
}

/** A part of the day, for a greeting. */
export function partOfDay(now = new Date()): 'morning' | 'afternoon' | 'evening' {
  const h = now.getHours();
  return h < 12 ? 'morning' : h < 18 ? 'afternoon' : 'evening';
}

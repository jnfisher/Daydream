// A scatter's arithmetic: an axis with round ticks, and records that share a spot drawn as
// one dot.
//
// Pure: no DOM, so `node --test` runs its tests (scatter.test.ts).

/** An axis from `lo` to `hi` widened to round numbers, with about `count` ticks between. */
export function niceAxis(lo: number, hi: number, count = 4): { lo: number; hi: number; ticks: number[] } {
  if (!Number.isFinite(lo) || !Number.isFinite(hi)) return { lo: 0, hi: 1, ticks: [0, 1] };
  if (hi < lo) [lo, hi] = [hi, lo];
  if (hi === lo) {
    lo -= 1;
    hi += 1;
  }
  const mag = 10 ** Math.floor(Math.log10((hi - lo) / count));
  // the finest round step that keeps to about `count` ticks
  const step = [1, 2, 5, 10, 20].map((f) => f * mag).find((s) => (hi - lo) / s <= count + 1)!;
  // decimals the step has, so 0.2 steps don't drift to 0.6000000000000001
  const dec = Math.max(0, -Math.floor(Math.log10(step) + 1e-9));
  const round = (v: number) => Number(v.toFixed(dec));
  const from = round(Math.floor(lo / step) * step);
  const to = round(Math.ceil(hi / step) * step);
  const ticks: number[] = [];
  for (let i = 0; i <= Math.round((to - from) / step); i++) ticks.push(round(from + i * step));
  return { lo: from, hi: to, ticks };
}

export interface Spot<P> {
  x: number;
  y: number;
  /** Every record at this spot, in the order they came. */
  all: P[];
}

/** The points by where they fall: records with the same two numbers share one dot. */
export function spots<P extends { x: number; y: number }>(points: readonly P[]): Spot<P>[] {
  const at = new Map<string, Spot<P>>();
  for (const p of points) {
    const k = `${p.x}/${p.y}`;
    const s = at.get(k);
    if (s) s.all.push(p);
    else at.set(k, { x: p.x, y: p.y, all: [p] });
  }
  return [...at.values()];
}

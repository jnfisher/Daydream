// The stamps saved from the map. Select saves the rooms picked as one (their shape: the cells the
// map draws them in, and the exits between them), and the stamp drawer holds them after its own
// shapes. They are kept beside the working copy (`.daydream/stamps.json`), as the map's places
// are, never in the game's data, and saving or deleting one is undone as a room moved by hand is.
// The Select tool's code and the stamps' both use them (each loads with its panel), so they are
// kept here, in neither.

import { api } from '../api';
import { sfx } from '../shell/prefs.svelte';
import { localStep, undo } from '../shell/state.svelte';
import { messageOf, toast } from '../shell/toasts.svelte';
import type { SavedStamp } from './model';
import type { Shape, StampKind } from './stamps';

/** A saved stamp's number, from its id (`s12` is 12), for their order. */
const numberOf = (id: string) => Number(id.slice(1)) || Infinity;

/** A shape as the server keeps it. */
export const wire = (s: Shape): Pick<SavedStamp, 'cells' | 'links'> => ({
  cells: s.cells,
  links: s.links.map((l) => (l.oneWay ? { a: l.a, dir: l.dir, b: l.b, one_way: true } : { a: l.a, dir: l.dir, b: l.b })),
});

/** A saved stamp's shape. */
export const shapeOfSaved = (s: SavedStamp): Shape => ({
  cells: s.cells,
  links: s.links.map((l) => (l.one_way ? { a: l.a, dir: l.dir, b: l.b, oneWay: true } : { a: l.a, dir: l.dir, b: l.b })),
});

/** How many rooms and exits a saved stamp makes, for a person: `24 rooms and 46 exits`. */
export function sizeOf(s: Pick<SavedStamp, 'cells' | 'links'>): string {
  const exits = s.links.reduce((n, l) => n + (l.one_way ? 1 : 2), 0);
  const rooms = s.cells.length;
  return `${rooms === 1 ? 'one room' : `${rooms.toLocaleString()} rooms`} and ${exits === 0 ? 'no exits' : exits === 1 ? 'one exit' : `${exits.toLocaleString()} exits`}`;
}

const kinds = new WeakMap<SavedStamp, StampKind>();

/** A saved stamp as the drawer holds it, beside its own shapes. */
export function kindOf(s: SavedStamp): StampKind {
  let k = kinds.get(s);
  if (!k) {
    const shape = shapeOfSaved(s);
    k = {
      id: `saved:${s.id}`,
      name: s.name,
      about: `${s.map ? `Saved from map ${s.map}` : 'Saved'}: ${sizeOf(s)}`,
      sizes: [],
      saved: s.id,
      ...(s.map && s.room ? { brush: { map: s.map, room: s.room } } : {}),
      make: () => shape,
    };
    kinds.set(s, k);
  }
  return k;
}

class SavedStamps {
  /** In the order they were first saved. */
  list = $state.raw<SavedStamp[]>([]);
  /** Read since the page opened (another world opened opens the page afresh). */
  loaded = $state(false);

  async load(): Promise<void> {
    try {
      this.list = (await api.savedStamps()).stamps;
      this.loaded = true;
    } catch (e) {
      toast(messageOf(e), { kind: 'error' });
    }
  }

  /** A stamp as the server answers it, in its place. */
  private put(s: SavedStamp) {
    this.list = [...this.list.filter((o) => o.id !== s.id), s].sort((a, b) => numberOf(a.id) - numberOf(b.id));
  }

  private take(id: string) {
    this.list = this.list.filter((o) => o.id !== id);
  }

  /** Saves a shape as a new stamp (from `map`, its new rooms copying `room` there); Ctrl+Z (or the toast) deletes it again. Answers it, or null when it could not be saved. */
  async add(name: string, shape: Shape, map: number, room: number): Promise<SavedStamp | null> {
    try {
      const r = await api.saveStamp({ name, map, room, ...wire(shape) });
      const s = r.stamp;
      this.put(s);
      localStep({
        label: `Save the stamp “${s.name}”`,
        after: r.step,
        undo: async () => {
          await api.dropStamp(s.id);
          this.take(s.id);
        },
        redo: async () => this.put((await api.saveStamp(s)).stamp),
      });
      sfx('pop');
      toast(`Saved the stamp “${s.name}” (${sizeOf(s)}): it is with the stamps now.`, { action: 'Undo', onAction: () => void undo() });
      return s;
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
      return null;
    }
  }

  /** Deletes a stamp; Ctrl+Z (or the toast) brings it back, where it was. */
  async remove(s: SavedStamp): Promise<boolean> {
    try {
      const r = await api.dropStamp(s.id);
      this.take(s.id);
      localStep({
        label: `Delete the stamp “${s.name}”`,
        after: r.step,
        undo: async () => this.put((await api.saveStamp(s)).stamp),
        redo: async () => {
          await api.dropStamp(s.id);
          this.take(s.id);
        },
      });
      sfx('toggle');
      toast(`Deleted the stamp “${s.name}”.`, { action: 'Undo', onAction: () => void undo() });
      return true;
    } catch (e) {
      sfx('nope');
      toast(messageOf(e), { kind: 'error' });
      return false;
    }
  }
}

export const saved = new SavedStamps();

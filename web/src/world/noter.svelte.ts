// The builder's notes on the map shown, and the one being written: a note on a room (perhaps a
// thing to do, ticked off when done), a label by a room, or a region drawn around rooms picked.
// Each change is saved to the map's side file at once (in the order made) and Ctrl+Z takes it
// back, like a room moved by hand: notes are never the game's, so the realm's history does not
// count them. Its code loads with the notes' (noting.ts).

import { api } from '../api';
import { sfx } from '../shell/prefs.svelte';
import { dropLocalStep, localStep, undo, type LocalStep } from '../shell/state.svelte';
import { dismiss, messageOf, toast } from '../shell/toasts.svelte';
import type { Host } from './host';
import { cellKey } from './layout';
import type { MapNote, NoteColour, NoteKind } from './model';
import { NoteLayer } from './notelayer';
import { newId, noteOn, toDo, topLeft } from './notes';

/** A note being written: a copy of the one it changes, or a new one (`fresh`). */
export interface Draft {
  map: number;
  id: string;
  kind: NoteKind;
  text: string;
  rooms: number[];
  colour?: NoteColour;
  todo: boolean;
  done: boolean;
  fresh: boolean;
}

export class Noter {
  readonly host: Host;
  readonly layer = new NoteLayer();
  /** The map shown's notes, and which map that is. */
  notes = $state.raw<MapNote[]>([]);
  map = $state(0);
  /** The note being written, if one is. */
  draft = $state<Draft | null>(null);
  /** The camera moved: the note being written follows its room. */
  moved = $state(0);
  /** The things left to do on the map shown. */
  todos = $derived(this.notes.filter(toDo).length);
  /** Changes sent and not yet answered: a read of the map meanwhile keeps these. */
  private pending = 0;
  private chain: Promise<unknown> = Promise.resolve();
  private toastId = 0;

  constructor(host: Host) {
    this.host = host;
  }

  /** The map read: its notes, unless changes to them are still on their way. */
  load(map: number, notes: readonly MapNote[]): void {
    if (map === this.map && this.pending) return;
    // another map: the note being written is kept on its own
    if (map !== this.map) this.finish(true);
    this.map = map;
    this.show(map, [...notes]);
  }

  private show(map: number, notes: MapNote[]): void {
    if (map !== this.map) return;
    this.notes = notes;
    this.layer.notes = notes;
    this.host.scene.draw();
  }

  /** A note by its id, on the map shown. */
  byId(id: string): MapNote | undefined {
    return this.notes.find((n) => n.id === id);
  }

  /** The room a note is shown at: its room, or the room its region's name is on (on the level shown, if any is). */
  roomOf(n: Pick<MapNote, 'kind' | 'rooms'>): number {
    if (n.kind !== 'region') return n.rooms[0]!;
    const l = this.host.scene.layout;
    const first = l && topLeft(n.rooms.flatMap((r) => l.at.get(r) ?? []));
    return (first && l.cell.get(cellKey(first.x, first.y))) ?? n.rooms[0]!;
  }

  // ---- writing -------------------------------------------------------------------------

  /** Opens room `n`'s note or label to write (a new one if it has none). */
  write(n: number, kind: 'note' | 'label'): void {
    if (this.draft?.kind === kind && this.draft.rooms[0] === n) return;
    const there = noteOn(this.notes, n, kind);
    if (there) return this.edit(there.id);
    this.begin({ map: this.map, id: newId(), kind, text: '', rooms: [n], todo: false, done: false, fresh: true });
  }

  /** A new region around these rooms, to name. */
  region(rooms: readonly number[]): void {
    if (!rooms.length) return;
    this.begin({ map: this.map, id: newId(), kind: 'region', text: '', rooms: [...rooms], todo: false, done: false, fresh: true });
  }

  /** Opens a note to change. */
  edit(id: string): void {
    if (this.draft?.id === id) return;
    const n = this.byId(id);
    if (!n) return;
    this.begin({ map: this.map, id, kind: n.kind, text: n.text, rooms: [...n.rooms], colour: n.colour, todo: !!n.todo, done: !!n.done, fresh: false });
  }

  /** Opens a note to write, keeping the one being written first. */
  private begin(d: Draft): void {
    this.finish(true);
    this.draft = d;
    this.layer.open = d.id;
    this.host.scene.draw();
  }

  /** The note being written put away: saved (a new note, a change, or with no words taken away), or not. */
  finish(save: boolean): void {
    const d = this.draft;
    if (!d) return;
    this.draft = null;
    this.layer.open = null;
    this.host.scene.draw();
    if (!save || d.map !== this.map) return;
    const text = d.text.trim();
    const was = this.byId(d.id) ?? null;
    if (!text) {
      if (was) this.remove(was.id);
      return;
    }
    const n: MapNote = { id: d.id, kind: d.kind, text, rooms: d.rooms };
    if (d.colour) n.colour = d.colour;
    if (d.kind === 'note' && d.todo) {
      n.todo = true;
      if (d.done) n.done = true;
    }
    if (was && JSON.stringify(was) === JSON.stringify(n)) return;
    const what = this.what(n);
    if (!was) this.change(null, n, `Add ${what}`, `Added ${what}.`);
    else if (!!was.done !== !!n.done && was.text === n.text) this.change(was, n, n.done ? `Tick off ${what}` : `Untick ${what}`, n.done ? `Ticked off ${what}.` : `${cap(what)} is to do again.`);
    else this.change(was, n, `Change ${what}`, `Changed ${what}.`);
  }

  /** Takes a note away. */
  remove(id: string): void {
    const was = this.byId(id);
    if (!was) return;
    if (this.draft?.id === id) {
      this.draft = null;
      this.layer.open = null;
    }
    const what = this.what(was);
    this.change(was, null, `Delete ${what}`, `Deleted ${what}.`);
  }

  /** Ticks a thing to do off, or makes it to do again. */
  tick(id: string, done: boolean): void {
    const was = this.byId(id);
    if (!was?.todo || !!was.done === done) return;
    const n: MapNote = { ...was };
    if (done) n.done = true;
    else delete n.done;
    const what = this.what(n);
    this.change(was, n, done ? `Tick off ${what}` : `Untick ${what}`, done ? `Ticked off ${what}.` : `${cap(what)} is to do again.`);
    if (this.draft?.id === id) this.draft.done = done;
  }

  /** A region drawn around other rooms. */
  reshape(id: string, rooms: readonly number[]): void {
    const was = this.byId(id);
    if (!was || was.kind !== 'region' || !rooms.length) return;
    const n: MapNote = { ...was, rooms: [...rooms] };
    if (this.draft?.id === id) this.draft.rooms = [...rooms];
    this.change(was, n, `Redraw ${this.what(n)}`, `${cap(this.what(n))} goes around ${count(rooms.length, 'room')} now.`);
  }

  /** A note as said: “the note on Town square 1/245”, “the region Docks”. */
  what(n: Pick<MapNote, 'kind' | 'text' | 'rooms' | 'todo'>): string {
    if (n.kind === 'region') return `the region “${clip(n.text, 40)}”`;
    if (n.kind === 'label') return `the label “${clip(n.text, 40)}”`;
    return `the ${n.todo ? 'thing to do' : 'note'} on ${this.host.called(n.rooms[0]!)}`;
  }

  // ---- saving --------------------------------------------------------------------------

  /** One note changed (from `before` to `after`; null: none), shown at once and saved: Ctrl+Z or its toast takes it back. */
  private change(before: MapNote | null, after: MapNote | null, label: string, said: string): void {
    const m = this.map;
    const id = (after ?? before)!.id;
    this.put(m, id, after);
    const step: LocalStep = {
      label,
      after: Infinity,
      undo: async () => {
        dismiss(this.toastId);
        await this.save(m, id, before, true);
      },
      redo: async () => {
        dismiss(this.toastId);
        await this.save(m, id, after, true);
      },
    };
    localStep(step);
    sfx('pop');
    dismiss(this.toastId);
    this.toastId = toast(said, { action: 'Undo', onAction: () => void undo() });
    this.host.tell(said);
    this.save(m, id, after, false).then(
      (newest) => (step.after = newest),
      (e) => {
        dropLocalStep(step);
        this.put(m, id, before);
        sfx('nope');
        dismiss(this.toastId);
        toast(messageOf(e), { kind: 'error' });
      },
    );
  }

  /** Puts a note in its place on the map shown (null: takes it away), as it will be saved. */
  private put(m: number, id: string, n: MapNote | null): void {
    if (m !== this.map) return;
    const i = this.notes.findIndex((x) => x.id === id);
    const next = [...this.notes];
    if (n && i >= 0) next[i] = n;
    else if (n) next.push(n);
    else if (i >= 0) next.splice(i, 1);
    this.show(m, next);
  }

  /** Saves a note (null: takes it away) after the changes sent before, shown too if `show`; answers the realm history's newest step. */
  private save(m: number, id: string, n: MapNote | null, show: boolean): Promise<number> {
    if (show) this.put(m, id, n);
    this.pending++;
    const done = this.chain.then(async () => {
      try {
        return (await api.notes(m, n ? [n] : [], n ? [] : [id])).step;
      } finally {
        this.pending--;
      }
    });
    this.chain = done.catch(() => {});
    return done;
  }

  // ---- what the pointer is over ----------------------------------------------------------

  /** The tooltip for a note pointed at. */
  tip(id: string): { title: string; sub: string } {
    const n = this.byId(id);
    if (!n) return { title: 'A note', sub: '' };
    if (n.kind === 'region') return { title: n.text, sub: `Your region of ${count(n.rooms.length, 'room')}. Click to change it.` };
    const room = this.host.called(n.rooms[0]!);
    if (n.kind === 'label') return { title: n.text, sub: `Your label on ${room}. Click to change it.` };
    const head = n.todo ? (n.done ? 'Done' : 'To do') : 'Your note';
    return { title: clip(n.text, 200), sub: `${head} on ${room}. Click to ${n.todo && !n.done ? 'change it or tick it off' : 'change it'}.` };
  }

  /** What a screen reader hears of a room's notes when it is selected. */
  said(n: number): string {
    const parts: string[] = [];
    const note = noteOn(this.notes, n, 'note');
    const label = noteOn(this.notes, n, 'label');
    if (label) parts.push(`Labelled ${label.text}.`);
    if (note) parts.push(`${note.todo ? (note.done ? 'Done' : 'To do') : 'Note'}: ${note.text}`);
    for (const r of this.notes) if (r.kind === 'region' && r.rooms.includes(n)) parts.push(`In the region ${r.text}.`);
    return parts.join(' ');
  }
}

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
const clip = (s: string, n: number) => (s.length > n ? `${s.slice(0, n - 1).trimEnd()}…` : s);
const count = (n: number, one: string) => `${n.toLocaleString()} ${one}${n === 1 ? '' : 's'}`;

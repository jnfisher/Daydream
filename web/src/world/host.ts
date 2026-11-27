// What the World page lends the map's tools, whose code loads the first time each is used (the
// stamps, the Select tool): its scene, the map shown, and its ways of showing an edit at once
// and saving it in turn. Types only, so it costs the World nothing.

import type { Spot } from './layout';
import type { ColourBy } from './look';
import type { MapRoom, PinCell } from './model';
import type { Scene } from './scene';

export interface Host {
  readonly scene: Scene;
  readonly canvas: HTMLCanvasElement;
  /** The map shown, and how many rooms it has. */
  readonly map: number;
  readonly count: number;
  /** The room selected: its card is open, and Build mode works from it. */
  readonly selected: MapRoom | null;
  /** How much of the map's right side the room card covers, in pixels. */
  readonly card: number;
  /** Where this map's rooms moved by hand are drawn (as it is drawn now: flat, or in levels). */
  readonly pins: ReadonlyMap<number, Spot>;
  /** The longest name a room takes. */
  readonly nameMax: number;
  /** A finger, not a mouse. */
  readonly coarse: boolean;
  /** A line in the hint bar for a moment (a toast on a small screen, unless `quiet`: then for a screen reader only). */
  say(text: string, quiet?: boolean): void;
  /** A line for a screen reader only. */
  tell(text: string): void;
  /** A room for a sentence: its name and number ("Town square 1/245"), or "room 1/245". */
  called(n: number): string;
  /** Shows the map's rooms as an edit leaves them, before the server has them. */
  patch(rooms: MapRoom[]): void;
  /**
   * Sends an edit once those before it are in. `done` says what it did (from its answer), in a
   * toast that can undo it; if it fails, those queued after it are not sent, `failed` puts back
   * what was shown, and the map is read again.
   */
  send<T>(work: () => Promise<T>, done: string | ((out: T) => string), failed?: () => void): void;
  /** Draws map `m`'s rooms in these places, if it is the map shown (in levels if they name their levels). */
  showPins(m: number, next: ReadonlyMap<number, Spot>): void;
  /** Saves places of map `m`'s rooms (all of them, if `replace`), after those sent before; answers the realm history's newest step. */
  savePins(m: number, body: Record<string, PinCell | null>, replace: boolean): Promise<number>;
  /** Takes the last edit's toast away: its Undo would undo something else now. */
  quiet(): void;
  /** Colours the map's rooms by this (Paint shows what it puts on rooms this way). */
  colour(by: ColourBy): void;
  /** Opens the stamps holding a stamp saved from the map (by its id), to put down. */
  stamp(saved: string): void;
}

// The parts of a record card only some records need (a monster's attacks, how it is wired,
// where it spawns; a weapon's scatter; what a text block does; what players can type in a
// room; how the game reads a message), loaded the first time a card shows one.

export { default as Attacks } from './Attacks.svelte';
export { default as Reads } from './Reads.svelte';
export { default as Scatter } from './Scatter.svelte';
export { default as Script } from './Script.svelte';
export { default as Spawns } from './Spawns.svelte';
export { default as Typed } from './Typed.svelte';
export { default as Wired } from './Wired.svelte';

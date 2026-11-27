// The notes' code (what a note is, how the map draws them, the list of a map's notes and the
// note being written), loaded the first time a map with notes is drawn or a note is written, so
// the World's own code stays small.

export { default as NoteEditor } from './NoteEditor.svelte';
export { default as NotesPanel } from './NotesPanel.svelte';
export { Noter } from './noter.svelte';

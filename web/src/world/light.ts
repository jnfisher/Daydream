// The game's words for how light a room is, for the map's colours and the Select tool's light
// field. No imports, so the tests and both of those can share it.

/** The game's words for a room's light (silvermere's display::light, as MajorMUD says it), each from its least level. */
export const LIGHT_WORDS: [number, string][] = [
  [900, 'Blinding'],
  [200, 'Daylight'],
  [0, 'Regular light'],
  [-100, 'Dimly lit'],
  [-150, 'Barely visible'],
  [-200, 'Very dark'],
  [-Infinity, 'Pitch black'],
];

/** The words for a light level. */
export const lightWord = (v: number) => LIGHT_WORDS.find(([from]) => v >= from)![1];

/** A level in each band below blinding, for the bar's quick picks. */
export const LIGHT_PICKS = [200, 0, -50, -125, -175, -250];

// The games Daydream found on this machine (MajorMUD's WCC files, MBBSEmu's .db files,
// silvermere databases), which one is open, whether each is running, and opening another, over
// the server's /api/games routes (crates/daydream-server/src/games). The Worlds page shows them
// (shell/Worlds.svelte).

export interface Current {
  id?: string;
  kind?: 'wcc' | 'db' | 'silvermere';
  label?: string;
  location?: string;
  live?: boolean;
  /** The world is worked on in a copy of the game's files, not in them. */
  copy?: boolean;
  realm_dir: string;
}

export interface Game {
  id: string;
  kind: 'wcc' | 'db' | 'silvermere';
  label: string;
  location: string;
  found_by: string;
  live: boolean;
  copy: { dir: string; pulled: string } | null;
  current: boolean;
}

/** A place Daydream looked for games, and the games it gave (by label). */
export interface Looked {
  where: string;
  kind: 'folder' | 'setting' | 'copies';
  /** Folders: how many levels down, and whether given with --search. */
  depth?: number;
  given?: boolean;
  found: string[];
}

export interface Listing {
  current: Current | null;
  games: Game[];
  looked: Looked[];
  custom: boolean;
  workspace: string;
}

export interface Status {
  running: boolean | null;
  by?: string | null;
  elsewhere: string[];
  error?: string;
}

async function call<T>(method: string, url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    headers: body === undefined ? {} : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (res.status === 401) location.href = '/login';
  const text = await res.text();
  let json: any = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    // an error page
  }
  if (!res.ok) throw new Error(json?.error ?? (text || res.statusText));
  return json as T;
}

export const games = {
  list: (refresh = false) => call<Listing>('GET', `/api/games${refresh ? '?refresh=true' : ''}`),
  status: (id: string) => call<Status>('GET', `/api/games/status?id=${encodeURIComponent(id)}`),
  open: (body: Record<string, unknown>) => call<{ current: Current }>('POST', '/api/games/open', body),
};

export const KINDS: Record<Game['kind'], string> = {
  wcc: 'MajorMUD (.DAT files)',
  db: 'MajorMUD (.db files)',
  silvermere: 'silvermere',
};

/** How a game is worked on, in a line; `copy`: in a working copy, not its own files. */
export function how(g: { kind: Game['kind']; live: boolean; copy?: boolean }): string {
  switch (g.kind) {
    case 'wcc':
      return 'Edited in a working copy while the game runs; Publish writes it into the game, through its update file or with the game stopped.';
    case 'silvermere':
      return 'Edited in a working copy while silvermere runs; Publish writes it into silvermere, which stops for a moment.';
    case 'db':
      if (g.copy) return 'Edited in a working copy while the game runs; Publish writes it into the game’s .db files with the game stopped.';
      return g.live
        ? 'The game’s own files: every save goes straight into the game, so stop it while editing.'
        : 'A realm folder, edited where it is.';
  }
}

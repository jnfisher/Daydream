// The game picker: the games Daydream found on this machine (MajorMUD's WCC files, MBBSEmu's
// .db files, silvermere databases), which one is open, whether each is running, and opening
// another, over the server's /api/games routes (crates/daydream-server/src/games).

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

interface Game {
  id: string;
  kind: 'wcc' | 'db' | 'silvermere';
  label: string;
  location: string;
  found_by: string;
  live: boolean;
  copy: { dir: string; pulled: string } | null;
  current: boolean;
}

interface Listing {
  current: Current | null;
  games: Game[];
  custom: boolean;
  workspace: string;
}

interface Status {
  running: boolean | null;
  by?: string | null;
  elsewhere: string[];
  error?: string;
}

function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Record<string, string | undefined> = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined) continue;
    if (k === 'class') el.className = v;
    else el.setAttribute(k, v);
  }
  el.append(...children);
  return el;
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

/** How a game is worked on, in a line. */
export function how(g: { kind: Game['kind']; live: boolean; copy?: boolean }): string {
  switch (g.kind) {
    case 'wcc':
      return 'Edited in a working copy; Publish writes it back into the game. Stop the game for that.';
    case 'silvermere':
      return 'Edited in a working copy; Publish writes it back into silvermere. Stop silvermere for that.';
    case 'db':
      if (g.copy) return 'Edited in a working copy while the game runs; Publish writes it into the game’s .db files. Stop the game for that.';
      return g.live
        ? 'The game’s own files: every save goes straight into the game, so stop it while editing.'
        : 'A realm folder, edited where it is.';
  }
}

function when(rfc3339: string): string {
  const d = new Date(rfc3339);
  return Number.isNaN(d.getTime()) ? rfc3339 : d.toLocaleString();
}

/** A running-state badge, filled in when the server answers. */
function statusBadge(id: string): HTMLElement {
  const el = h('span', { class: 'badge' }, 'checking…');
  games
    .status(id)
    .then((s) => {
      if (s.running === null) {
        el.textContent = 'cannot tell';
        el.title = s.error ?? '';
        el.className = 'badge';
      } else if (s.running) {
        el.textContent = 'running';
        el.title = s.by ?? '';
        el.className = 'badge running';
      } else {
        el.textContent = 'stopped';
        el.className = 'badge stopped';
        if (s.elsewhere.length) {
          el.textContent = 'stopped here';
          el.title = `Running elsewhere on this machine: ${s.elsewhere.join('; ')}`;
        }
      }
    })
    .catch((e) => {
      el.textContent = 'cannot tell';
      el.title = e instanceof Error ? e.message : String(e);
    });
  return el;
}

/** The games page. `opened` runs once another game is open. */
export function gamesPage(opened: () => void): HTMLElement {
  const page = h('div', { class: 'tools games' });
  const failure = (e: unknown) => h('p', { class: 'error' }, e instanceof Error ? e.message : String(e));

  const open = async (body: Record<string, unknown>, what: string, button?: HTMLButtonElement) => {
    if (button) {
      button.disabled = true;
      button.textContent = body.refresh ? 'Reading…' : 'Opening…';
    }
    try {
      await games.open(body);
      opened();
    } catch (e) {
      if (button) {
        button.disabled = false;
        button.textContent = body.refresh ? 'Read again' : 'Open';
      }
      alert(`${what}: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const render = (l: Listing) => {
    page.replaceChildren();
    const cur = h('section', { class: 'tool' }, h('h2', {}, 'Open now'));
    if (l.current?.label) {
      const c = l.current;
      cur.append(
        h('p', {}, h('strong', {}, c.label!), ' ', c.id ? statusBadge('current') : ''),
        h('p', { class: 'hint' }, `${KINDS[c.kind!]}: ${c.location}`),
        h('p', { class: 'hint' }, how({ kind: c.kind!, live: !!c.live, copy: !!c.copy })),
      );
      if (c.kind !== 'db' || c.copy) cur.append(h('p', { class: 'hint' }, `Working copy: ${c.realm_dir}`));
    } else if (l.current) {
      cur.append(h('p', {}, l.current.realm_dir));
    } else {
      cur.append(h('p', {}, 'No game is open yet: choose one below.'));
    }
    page.append(cur);

    const found = h('section', { class: 'tool' }, h('h2', {}, 'Games on this machine'));
    const again = h('button', { type: 'button' }, 'Look again');
    again.addEventListener('click', () => {
      again.disabled = true;
      again.textContent = 'Looking…';
      games
        .list(true)
        .then(render)
        .catch((e) => page.append(failure(e)));
    });
    found.append(
      h(
        'p',
        { class: 'hint' },
        'Daydream looks for MajorMUD’s WCC files (Worldgroup, The Major BBS, MBBSEmu) and silvermere databases where they are usually installed on the machine it runs on, and in the folders given with --search.',
      ),
    );
    if (!l.games.length) found.append(h('p', {}, 'None found. Start Daydream with --search FOLDER to look somewhere else.'));
    const table = h('table', { class: 'grid list games-list' });
    table.append(h('thead', {}, h('tr', {}, ...['Game', 'Where', 'State', ''].map((c) => h('th', {}, c)))));
    const body = h('tbody');
    for (const g of l.games) {
      const openBtn = h('button', { type: 'button', class: g.current ? undefined : 'primary' }, 'Open');
      openBtn.disabled = g.current;
      openBtn.addEventListener('click', () => void open({ id: g.id }, g.label, openBtn));
      const actions = h('div', { class: 'toolbar' }, openBtn);
      if (g.copy) {
        const pull = h('button', { type: 'button' }, 'Read again');
        pull.title = 'Read the game again into its working copy, replacing it (the copy it replaces is kept beside it, as .previous)';
        pull.addEventListener('click', () => {
          if (
            !confirm(
              `Read ${g.label} again into its working copy? Edits in the copy that were not written back to the game are set aside (kept as ${g.copy!.dir}.previous).`,
            )
          )
            return;
          void open({ id: g.id, refresh: true }, g.label, pull);
        });
        actions.append(pull);
      }
      const where = h('td', {}, g.location, h('div', { class: 'hint' }, `${KINDS[g.kind]}; found by ${g.found_by}`));
      if (g.copy) where.append(h('div', { class: 'hint' }, `Working copy read ${when(g.copy.pulled)}`));
      const name = h('td', {}, g.label);
      if (g.current) name.append(h('div', { class: 'hint' }, 'open now'));
      body.append(h('tr', { class: g.current ? 'current' : undefined }, name, where, h('td', {}, statusBadge(g.id)), h('td', {}, actions)));
    }
    table.append(body);
    if (l.games.length) found.append(table);
    found.append(h('div', { class: 'toolbar' }, again));
    page.append(found);

    const other = h('section', { class: 'tool' }, h('h2', {}, 'Open another'));
    if (l.custom) {
      const folder = h('input', { type: 'text', placeholder: 'C:\\WGSERV, or a folder of .db files' });
      const goFolder = h('button', { type: 'button' }, 'Open folder');
      goFolder.addEventListener('click', () => void open({ kind: 'folder', location: folder.value }, folder.value, goFolder));
      const db = h('input', { type: 'text', placeholder: 'C:\\silvermere\\data\\silvermere.db, or postgres://user:password@host/db' });
      const goDb = h('button', { type: 'button' }, 'Open silvermere');
      goDb.addEventListener('click', () => void open({ kind: 'silvermere', location: db.value }, 'silvermere', goDb));
      other.append(
        h('p', { class: 'hint' }, 'A game folder or a silvermere database Daydream did not find: a folder on the machine Daydream runs on, or a PostgreSQL URL (Neon and other hosts).'),
        h('label', { class: 'path' }, 'Game folder ', folder, goFolder),
        h('label', { class: 'path' }, "silvermere's database ", db, goDb),
        h('p', { class: 'hint' }, `Working copies are kept in ${l.workspace}.`),
      );
    } else {
      other.append(
        h(
          'p',
          { class: 'hint' },
          'This Daydream is reached with an access token, so it opens only the games it found itself. To add one, start it with --search FOLDER (or --allow-any-source to allow typing any folder or database here).',
        ),
      );
    }
    page.append(other);
  };

  games
    .list()
    .then(render)
    .catch((e) => page.append(failure(e)));
  return page;
}

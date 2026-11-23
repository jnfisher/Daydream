// NMR's Export to MMUD Explorer (frmMME_Export), laid out as NMR's form, over the server's
// /api/mme routes. The export is the library's (daydream_nmr::mme), the same the command
// line's `daydream mme-export` writes, so it reveals what NMR's export reveals and no more.
//
// NMR's "Export Only ..." boxes and "For GreaterMUD Engine" are not here: the library
// always writes the whole export, and Daydream supports 1.11p only.

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

interface Excluded {
  map: number;
  from: number;
  to: number;
}

interface Info {
  jet: boolean;
  dat_version: string;
  defaults: (Excluded & { name: string })[];
}

/** NMR's boxes, as `daydream_nmr::mme::MmeOptions` takes them. */
export interface MmeOptions {
  legit: boolean;
  disable_kai: boolean;
  exclude_default: boolean;
  exclude_rooms: boolean;
  excluded: Excluded[];
  hide_excluded: boolean;
  no_rooms: boolean;
  custom: string;
  update_url: string;
  dat_version: string;
}

/** NMR's config file (`SaveConfig`): an INI file's [Settings], boxes as 1 or 0. */
export function toIni(o: MmeOptions): string {
  const b = (v: boolean) => (v ? '1' : '0');
  const rooms = o.excluded.map((r) => `${r.map}/${r.from}/${r.to},`).join('');
  const lines = [
    '[Settings]',
    `CustomName=${o.custom}`,
    `Legit=${b(o.legit)}`,
    'Gmud=0',
    `DisableKai=${b(o.disable_kai)}`,
    `ExcludeDefault=${b(o.exclude_default)}`,
    `DB_Ver=${o.dat_version}`,
    `UpdateURL=${o.update_url}`,
    `EnableExcludeRooms=${b(o.exclude_rooms)}`,
    `HideExcludedRooms=${b(o.hide_excluded)}`,
    `NoRooms=${b(o.no_rooms)}`,
    `ExcludedRooms=${rooms || '0'}`,
  ];
  return lines.join('\r\n') + '\r\n';
}

/** Reads NMR's config file (`LoadConfig`) over `base`: what the file lacks stays as it was. */
export function fromIni(text: string, base: MmeOptions): MmeOptions {
  const settings = new Map<string, string>();
  let section = '';
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    const head = /^\[(.*)\]$/.exec(line);
    if (head) {
      section = head[1].toLowerCase();
      continue;
    }
    const eq = line.indexOf('=');
    if (section === 'settings' && eq > 0) settings.set(line.slice(0, eq).trim().toLowerCase(), line.slice(eq + 1));
  }
  const o = { ...base, excluded: [...base.excluded] };
  const s = (k: string) => settings.get(k.toLowerCase());
  const flag = (k: string, d: boolean) => {
    const v = s(k);
    return v === undefined ? d : v.trim() === '1';
  };
  // NMR writes "0" for an empty name, version or URL
  const text_ = (k: string, d: string) => {
    const v = s(k);
    return v === undefined || v === '0' ? d : v;
  };
  o.custom = text_('CustomName', o.custom);
  o.dat_version = text_('DB_Ver', o.dat_version);
  o.update_url = text_('UpdateURL', o.update_url);
  o.legit = flag('Legit', o.legit);
  o.disable_kai = flag('DisableKai', o.disable_kai);
  o.exclude_default = flag('ExcludeDefault', o.exclude_default);
  o.exclude_rooms = flag('EnableExcludeRooms', o.exclude_rooms);
  o.hide_excluded = flag('HideExcludedRooms', o.hide_excluded);
  o.no_rooms = flag('NoRooms', o.no_rooms);
  const rooms = s('ExcludedRooms');
  if (rooms !== undefined) {
    o.excluded = [];
    // "map/from/to," each; NMR reads only the entries that end in a comma
    for (const entry of rooms.split(',').slice(0, -1)) {
      const [map, from, to] = entry.split('/', 3).map((n) => Number(n.trim()));
      if ([map, from, to].every(Number.isFinite)) o.excluded.push({ map, from, to });
    }
  }
  return o;
}

/** NMR's default file name: `data-` and the BBS name without its spaces. */
export function fileName(o: MmeOptions, ext: string): string {
  const name = o.custom.replace(/ /g, '') || o.dat_version.replace(/ /g, '') || 'mme';
  return `data-${name}.${ext}`;
}

function save(blob: Blob, name: string) {
  const a = h('a', { href: URL.createObjectURL(blob), download: name });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 10_000);
}

async function exportAs(o: MmeOptions, format: 'summary' | 'plan' | 'mdb'): Promise<Response> {
  const res = await fetch('/api/mme/export', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ options: o, format }),
  });
  if (!res.ok) {
    const text = await res.text();
    let msg = text || res.statusText;
    try {
      msg = JSON.parse(text).error ?? msg;
    } catch {
      // not JSON
    }
    throw new Error(msg);
  }
  return res;
}

/** The Export to MMUD Explorer card. */
export function mmeExport(failure: (e: unknown) => HTMLElement): HTMLElement {
  const card = h('section', { class: 'tool mme' }, h('h2', {}, 'Export to MMUD Explorer'));
  card.append(
    h(
      'p',
      { class: 'hint' },
      "Writes the .mdb MMUD Explorer reads, as NMR's export does: the same tables and values, so it shows players what NMR's export shows and nothing more. It only reads the realm.",
    ),
  );
  const body = h('div', {}, 'Loading…');
  card.append(body);
  fetch('/api/mme')
    .then((r) => r.json() as Promise<Info>)
    .then((info) => body.replaceChildren(...form(info, failure)))
    .catch((e) => body.replaceChildren(failure(e)));
  return card;
}

function form(info: Info, failure: (e: unknown) => HTMLElement): Node[] {
  const checkbox = (label: string, checked = false) => {
    const i = h('input', { type: 'checkbox', checked: checked ? '' : undefined });
    return [h('label', {}, i, ' ', label), i] as const;
  };
  const text = (value = '', cls = 'text') => h('input', { type: 'text', value, class: cls });
  const num = () => h('input', { type: 'text', inputmode: 'numeric', class: 'number' });

  // config file
  const loadFile = h('input', { type: 'file', accept: '.ini,.txt' });
  const saveCfg = h('button', { type: 'button' }, 'Save Config ...');
  const config = h('div', { class: 'toolbar' }, h('label', { class: 'file' }, 'Load Config ... ', loadFile), saveCfg);

  // names
  const custom = text('');
  const datVersion = text(info.dat_version);
  const updateUrl = text('', 'text url');
  const names = h(
    'div',
    { class: 'mme-names' },
    h('label', {}, 'BBS/Database Name ', custom),
    h('label', {}, 'Dat Version/Realm ', datVersion),
    h('label', {}, 'Update Webpage (Help > Check for Updates in MME) ', updateUrl),
  );

  // export options
  const [legitL, legit] = checkbox('"Legit" Export', true);
  const [kaiL, kai] = checkbox('Disable Kai Auto-Learning');
  const [defL, def] = checkbox('Exclude Default "Not in game" Rooms', true);
  const listBtn = h('button', { type: 'button' }, 'List');
  const defaults = h('table', { class: 'grid list', hidden: '' });
  defaults.append(
    h('thead', {}, h('tr', {}, ...['Map', 'Start', 'End', 'Room'].map((c) => h('th', {}, c)))),
    h(
      'tbody',
      {},
      ...info.defaults.map((d) =>
        h('tr', {}, ...[d.map, d.from, d.to].map((v) => h('td', { class: 'n' }, String(v))), h('td', {}, d.name)),
      ),
    ),
  );
  listBtn.addEventListener('click', () => (defaults.hidden = !defaults.hidden));
  const options = h(
    'fieldset',
    {},
    h('legend', {}, 'Export Options'),
    h('div', {}, legitL),
    h('div', {}, kaiL),
    h('div', {}, defL, listBtn),
    defaults,
  );

  // exclude rooms
  const [enableL, enable] = checkbox('Enable');
  const [hideL, hide] = checkbox('Hide Only');
  const [allL, all] = checkbox('Exclude All');
  const map = num();
  const from = num();
  const to = num();
  const add = h('button', { type: 'button' }, 'Add -->');
  const remove = h('button', { type: 'button' }, 'Remove');
  const clear = h('button', { type: 'button' }, 'Clear');
  const list = h('select', { size: '6', multiple: '', class: 'mme-rooms' });
  const exclude = h(
    'fieldset',
    {},
    h('legend', {}, 'Exclude Rooms'),
    h('div', {}, enableL, hideL, allL),
    h(
      'div',
      { class: 'toolbar' },
      h('label', {}, 'Map: ', map),
      h('label', {}, 'Start ', from),
      h('label', {}, 'End ', to),
      add,
    ),
    list,
    h('div', { class: 'toolbar' }, remove, clear),
  );
  let excluded: Excluded[] = [];
  const showExcluded = () =>
    list.replaceChildren(
      ...excluded.map((r, i) => h('option', { value: String(i) }, `Map ${r.map}: ${r.from} - ${r.to}`)),
    );
  add.addEventListener('click', () => {
    const n = (i: HTMLInputElement) => Number(i.value.trim());
    const r = { map: n(map), from: n(from), to: to.value.trim() ? n(to) : n(from) };
    if (![r.map, r.from, r.to].every((v) => Number.isInteger(v) && v > 0) || r.to < r.from) {
      status.replaceChildren(failure('Give a map, and a start room no later than the end room.'));
      return;
    }
    status.replaceChildren();
    excluded.push(r);
    showExcluded();
    from.value = '';
    to.value = '';
    from.focus();
  });
  remove.addEventListener('click', () => {
    const picked = new Set([...list.selectedOptions].map((o) => Number(o.value)));
    excluded = excluded.filter((_, i) => !picked.has(i));
    showExcluded();
  });
  clear.addEventListener('click', () => {
    excluded = [];
    showExcluded();
  });

  // NMR's EnableLegit and ExcludeRoomsEnable: Legit turns off the custom name, URL and
  // the room list; the list's boxes and inputs follow Enable
  const sync = () => {
    const l = legit.checked;
    custom.disabled = l;
    updateUrl.disabled = l;
    if (l) enable.checked = false;
    enable.disabled = l;
    const on = enable.checked && !l;
    for (const el of [hide, all, map, from, to, add, remove, clear, list]) el.disabled = !on;
  };
  legit.addEventListener('change', sync);
  enable.addEventListener('change', sync);

  const options_ = (): MmeOptions => ({
    legit: legit.checked,
    disable_kai: kai.checked,
    exclude_default: def.checked,
    exclude_rooms: enable.checked,
    excluded: [...excluded],
    hide_excluded: hide.checked,
    no_rooms: all.checked,
    custom: custom.value,
    update_url: updateUrl.value,
    dat_version: datVersion.value,
  });
  const fill = (o: MmeOptions) => {
    legit.checked = o.legit;
    kai.checked = o.disable_kai;
    def.checked = o.exclude_default;
    enable.checked = o.exclude_rooms;
    hide.checked = o.hide_excluded;
    all.checked = o.no_rooms;
    custom.value = o.custom;
    updateUrl.value = o.update_url;
    datVersion.value = o.dat_version;
    excluded = [...o.excluded];
    showExcluded();
    sync();
  };
  sync();

  loadFile.addEventListener('change', async () => {
    const f = loadFile.files?.[0];
    if (!f) return;
    try {
      fill(fromIni(await f.text(), options_()));
      status.replaceChildren(h('p', {}, `Loaded ${f.name}.`));
    } catch (e) {
      status.replaceChildren(failure(e));
    }
    loadFile.value = '';
  });
  saveCfg.addEventListener('click', () => save(new Blob([toIni(options_())], { type: 'text/plain' }), 'mme-export.ini'));

  // export
  const preview = h('button', { type: 'button' }, 'Preview');
  const go = h('button', { type: 'button', class: 'primary' }, info.jet ? 'Export to MME' : 'Export plan');
  const script = h('a', { href: '/api/mme/write-mdb.ps1', download: 'write-mdb.ps1' }, 'write-mdb.ps1');
  const status = h('div', {});
  const how = info.jet
    ? h('p', { class: 'hint' }, 'The .mdb is written through Windows\' Jet engine on this machine and downloaded.')
    : h(
        'p',
        { class: 'hint' },
        "This machine has no Windows Jet engine, which writing a .mdb needs. Export plan downloads the export as a plan; on any Windows machine, put it beside ",
        script,
        ' and run ',
        h('code', {}, 'powershell -ExecutionPolicy Bypass -File write-mdb.ps1 data-NAME.plan data-NAME.mdb'),
        ' to write the .mdb MME reads.',
      );
  const buttons = [preview, go];
  const run = async (format: 'summary' | 'plan' | 'mdb') => {
    const o = options_();
    for (const b of buttons) b.disabled = true;
    status.replaceChildren(h('p', {}, format === 'summary' ? 'Building the export…' : 'Exporting… this walks the whole world and takes a little while.'));
    try {
      const res = await exportAs(o, format);
      if (format === 'summary') {
        const { tables } = (await res.json()) as { tables: { name: string; rows: number }[] };
        const t = h('table', { class: 'grid' });
        t.append(
          h('thead', {}, h('tr', {}, h('th', {}, 'Table'), h('th', {}, 'Rows'))),
          h('tbody', {}, ...tables.map((r) => h('tr', {}, h('td', {}, r.name), h('td', { class: 'n' }, String(r.rows))))),
        );
        status.replaceChildren(h('p', {}, 'Preview: what the export would hold. Nothing was written.'), t);
      } else {
        const name = fileName(o, format);
        save(await res.blob(), name);
        status.replaceChildren(h('p', {}, `Done: ${name}.`));
      }
    } catch (e) {
      status.replaceChildren(failure(e));
    } finally {
      for (const b of buttons) b.disabled = false;
    }
  };
  preview.addEventListener('click', () => void run('summary'));
  go.addEventListener('click', () => void run(info.jet ? 'mdb' : 'plan'));

  return [
    config,
    names,
    h('div', { class: 'mme-frames' }, options, exclude),
    how,
    h('div', { class: 'toolbar' }, preview, go),
    status,
  ];
}

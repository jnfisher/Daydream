// NMR's Quest Organizer: a tree of quests read from a quest file in NMR's format, each line
// naming a text block, its part and the text block it links to. NMR reads NMR-Quests.txt or
// NMR-QuestsCustom.txt from its own folder; here they are read from the realm's folder, or
// opened from this computer. Picking a line shows its text block, with links to edit it and
// the one it links to.

import { renderAnsi } from './ansi';

/** A node of the tree: a section, a comment or a text block line. */
export interface QuestNode {
  kind: 'section' | 'comment' | 'line';
  text: string;
  children: QuestNode[];
  /** A text block line's fields. */
  block?: number;
  part?: number;
  link?: number;
  description?: string;
}

/** NMR's `ExtractField`: the `n`th `|`-separated field, from 1. */
const field = (line: string, n: number) => line.split('|')[n - 1] ?? '';
/** VB's `Val`: the number a string starts with, else 0. */
const val = (s: string) => {
  const m = /^\s*[-+]?\d+/.exec(s);
  return m ? Number(m[0]) : 0;
};

/** Parses a quest file as NMR's `CreateTree` does: `#` lines and blank lines are skipped,
 *  `-`, `--` and `---` start a section, a sub-section of the last section and a
 *  sub-section of the last sub-section, `'` is a comment and anything else is
 *  `text block|part|links to|description`, under the last (sub-)section started. */
export function parseQuests(text: string): QuestNode {
  const root: QuestNode = { kind: 'section', text: 'Quests', children: [] };
  let section = root;
  let sub = root;
  let working = root;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw;
    if (line === '' || line.startsWith('#')) continue;
    if (line.startsWith('---')) {
      working = { kind: 'section', text: line.slice(3), children: [] };
      sub.children.push(working);
    } else if (line.startsWith('--')) {
      sub = working = { kind: 'section', text: line.slice(2), children: [] };
      section.children.push(sub);
    } else if (line.startsWith('-')) {
      section = sub = working = { kind: 'section', text: line.slice(1), children: [] };
      root.children.push(section);
    } else if (line.startsWith("'")) {
      working.children.push({ kind: 'comment', text: field(line, 1).slice(1), children: [] });
    } else {
      working.children.push({
        kind: 'line',
        text: `${field(line, 1)} - p${field(line, 2)} --> ${field(line, 3)}`,
        children: [],
        block: val(field(line, 1)),
        part: val(field(line, 2)),
        link: val(field(line, 3)),
        description: field(line, 4),
      });
    }
  }
  return root;
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

const editLink = (block: number, part: number, label: string) =>
  h('a', { href: `#/Textblocks/${encodeURIComponent(JSON.stringify([block, part]))}` }, label);

async function textblock(block: number, part: number): Promise<string | null> {
  const key = encodeURIComponent(JSON.stringify([block, part]));
  const res = await fetch(`/api/tables/Textblocks/row?key=${key}`);
  if (!res.ok) return null;
  const row = await res.json();
  const data = row?.values?.Data;
  return typeof data === 'string' ? data.split('\0')[0] : null;
}

/** The Quest Organizer card. */
export function questOrganizer(): HTMLElement {
  const which = h('select', { 'aria-label': 'Quest file' }, h('option', { value: 'default' }, 'Default'), h('option', { value: 'custom' }, 'Custom'));
  const load = h('button', { type: 'button', class: 'primary' }, 'Load');
  const pick = h('input', { type: 'file', accept: '.txt,text/plain', 'aria-label': 'Open a quest file' });
  const tree = h('div', { class: 'quest-tree' });
  const detail = h('div', { class: 'quest-detail' });
  const out = h('div', { class: 'tool-result' });

  const show = (text: string, from: string) => {
    const root = parseQuests(text);
    tree.replaceChildren(h('p', { class: 'note' }, from), branch(root, true));
    detail.replaceChildren(h('p', { class: 'note' }, 'Pick a line to see its text block.'));
    out.replaceChildren(h('div', { class: 'quest-body' }, tree, detail));
  };

  function branch(node: QuestNode, open = false): HTMLElement {
    if (node.kind === 'section') {
      const d = h('details', { class: 'quest-section' }, h('summary', {}, node.text));
      if (open) d.open = true;
      d.append(...node.children.map((c) => branch(c)));
      return d;
    }
    const b = h('button', { type: 'button', class: `quest-${node.kind}` }, node.kind === 'comment' ? `${node.text.slice(0, 40)}...` : node.text);
    b.addEventListener('click', () => {
      tree.querySelectorAll('.picked').forEach((x) => x.classList.remove('picked'));
      b.classList.add('picked');
      void pickNode(node);
    });
    return b;
  }

  async function pickNode(node: QuestNode) {
    if (node.kind === 'comment') {
      detail.replaceChildren(h('p', {}, node.text));
      return;
    }
    const { block = 0, part = 0, link = 0 } = node;
    const facts = h(
      'dl',
      {},
      h('dt', {}, 'Text block'),
      h('dd', {}, editLink(block, part, `${block}, part ${part}`)),
      h('dt', {}, 'Links to'),
      h('dd', {}, link ? editLink(link, 0, String(link)) : '0'),
    );
    const box = h('pre', { class: 'terminal', 'aria-label': 'Text block' }, 'Loading…');
    detail.replaceChildren(h('p', {}, node.description ?? ''), facts, box);
    const data = await textblock(block, part);
    box.replaceChildren(data === null ? `Text block ${block} part ${part} is not in the realm.` : renderAnsi(data));
  }

  load.addEventListener('click', async () => {
    out.replaceChildren(h('p', {}, 'Loading…'));
    try {
      const res = await fetch(`/api/quests?file=${which.value}`);
      const body = await res.json().catch(() => null);
      if (!res.ok) throw new Error(body?.error ?? res.statusText);
      show(body.text, `${body.name}, from the realm's folder`);
    } catch (e) {
      out.replaceChildren(h('p', { class: 'error' }, e instanceof Error ? e.message : String(e)));
    }
  });
  pick.addEventListener('change', async () => {
    const f = pick.files?.[0];
    if (!f) return;
    // NMR's quest files are Windows-1252
    const text = new TextDecoder('windows-1252').decode(await f.arrayBuffer());
    show(text, `${f.name}, from this computer`);
  });

  return h(
    'section',
    { class: 'tool' },
    h('h2', {}, 'Quest Organizer'),
    h(
      'p',
      { class: 'hint' },
      "NMR's quest notes: which text blocks make up each quest. Load NMR-Quests.txt or NMR-QuestsCustom.txt from the realm's folder, or open a quest file from this computer.",
    ),
    h('div', { class: 'toolbar' }, h('label', {}, 'File ', which), load, h('label', {}, 'or open ', pick)),
    out,
  );
}

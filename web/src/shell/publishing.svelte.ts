// Publishing, as the frame follows it: the plan for the open world, and the job under way or
// last finished. The job runs on the server; this asks how it goes until it ends, wherever the
// person is in the app, so the top bar's Publish button and the Publish page agree.

import Clock from '@lucide/svelte/icons/clock';
import Database from '@lucide/svelte/icons/database';
import FileText from '@lucide/svelte/icons/file-text';
import HardDrive from '@lucide/svelte/icons/hard-drive';
import HeartPulse from '@lucide/svelte/icons/heart-pulse';
import Megaphone from '@lucide/svelte/icons/megaphone';
import Play from '@lucide/svelte/icons/play';
import Power from '@lucide/svelte/icons/power';
import { api, type PublishJob, type PublishPlan, type PublishStep, type PublishWarn, type PublishWayId } from '../api';
import { sfx } from './prefs.svelte';
import { app, refreshNow, refreshStatus } from './state.svelte';
import { messageOf, toast } from './toasts.svelte';

export const pub = $state({
  plan: null as PublishPlan | null,
  /** Why the plan could not be read, when it could not. */
  error: '',
  job: null as PublishJob | null,
  /** The job's progress is shown over the page. */
  sheet: false,
});

/** Reads the plan again. */
export async function loadPlan(): Promise<void> {
  if (!app.current) return;
  try {
    pub.plan = await api.publishPlan();
    pub.error = '';
    // the job as the plan saw it, unless this page follows it already
    const job = pub.plan.job;
    if (job && !(timer && pub.job?.id === job.id)) pub.job = job;
    if (pub.job?.state === 'running') follow();
  } catch (e) {
    pub.error = messageOf(e);
  }
}

/** Each step's picture. */
export function stepIcon(step: PublishStep, way: PublishWayId): typeof Power {
  switch (step.id) {
    case 'check':
      return HeartPulse;
    case 'warn':
      return Megaphone;
    case 'stop':
      return Power;
    case 'backup':
      return HardDrive;
    case 'write':
      return way === 'update' ? FileText : Database;
    case 'start':
      return Play;
    default:
      return Clock;
  }
}

/** How long silvermere's players may be given before it closes, in minutes (0: now). */
export const WARN_MINUTES = [0, 1, 5, 15];

/** Why silvermere closes, unless the sysop says otherwise (the server's own default too). */
export const DEFAULT_MESSAGE = 'a world update, back in a minute or two';

/** What silvermere's players read first, `minutes` before it closes. */
export function warning(minutes: number, why: string): string {
  if (!minutes) return `The server shuts down now: ${why}`;
  return `The server will shut down in ${minutes} minute${minutes === 1 ? '' : 's'}: ${why}`;
}

/** The game an engine publishes into, as a sysop calls it. */
export const GAME: Record<string, string> = {
  worldgroup: 'Worldgroup',
  majorbbs: 'The Major BBS',
  mbbsemu: 'MBBSEmu',
  db: 'the BBS',
  silvermere: 'silvermere',
  files: 'the game',
};

let timer: ReturnType<typeof setTimeout> | null = null;

/** Asks how the job goes until it ends. */
function follow(): void {
  if (timer) return;
  const tick = async () => {
    timer = null;
    let job: PublishJob | null = null;
    try {
      job = await api.publishJob();
    } catch {
      // the server is busy, or restarting: ask again
    }
    if (job) pub.job = job;
    if (!job || job.state === 'running') {
      timer = setTimeout(() => void tick(), 700);
      return;
    }
    ended(job);
  };
  timer = setTimeout(() => void tick(), 400);
}

function ended(job: PublishJob): void {
  void refreshNow();
  void refreshStatus();
  void loadPlan();
  // written, but a step after it went wrong (silvermere did not come back)
  const troubled = job.steps.some((s) => s.state === 'failed');
  if (job.state === 'done') sfx(troubled ? 'nope' : 'tada');
  else if (job.state === 'failed') sfx('nope');
  // shown over the page: the sheet says it; elsewhere a toast does
  if (pub.sheet) return;
  const show = () => (pub.sheet = true);
  if (job.state === 'done') toast(job.result ?? 'Published.', { kind: troubled ? 'error' : 'info', action: 'See how it went', onAction: show, ms: 9000 });
  else if (job.state === 'failed') toast(`Publishing stopped: ${job.error ?? 'something went wrong'}`, { kind: 'error', action: 'Show me', onAction: show });
}

/** Starts publishing `way`; `warn`: how silvermere warns its players first. */
export async function publish(way: PublishWayId, warn?: PublishWarn): Promise<void> {
  try {
    pub.job = await api.publish(way, warn);
    pub.sheet = true;
    follow();
  } catch (e) {
    sfx('nope');
    toast(messageOf(e), { kind: 'error' });
    void loadPlan();
  }
}

/** Calls off a publish while it may be: silvermere's countdown, or waiting for the game to stop. */
export async function cancelPublish(): Promise<void> {
  try {
    const r = await api.cancelPublish();
    if (!r.cancelled) toast('It is past that now, so it carries on.');
  } catch (e) {
    toast(messageOf(e), { kind: 'error' });
  }
}

/** At start: a publish still under way (the page was reloaded) is followed again. */
export async function resumePublish(): Promise<void> {
  try {
    const job = await api.publishJob();
    if (job) pub.job = job;
    if (job?.state === 'running') follow();
  } catch {
    // no game open yet
  }
}

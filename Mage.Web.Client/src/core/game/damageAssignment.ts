import type { GameView, PermanentView } from '../../protocol/generated/views';
import { stripMarkup, type MultiAmountPrompt } from './prompt';

/**
 * Combat damage assignment, read from the server's multi-amount prompt. The server (CombatGroup) asks the attacking
 * player to divide an attacker's damage among its blockers (or a banding blocker's damage among attackers), one row
 * per creature in damage assignment order. Each row's message is the creature's log name, which carries its object id,
 * so the division can be drawn on the creatures themselves.
 */

export interface DamageRecipient {
  /** the creature's object id on the board */
  id: string;
  name: string;
  /** damage that destroys it right now (toughness left, or 1 against deathtouch); 0 when already lethal */
  lethal: number;
  min: number;
  max: number;
}

export interface DamageAssignment {
  /** the creature dealing the damage, when the header names it */
  sourceId: string | null;
  sourceName: string;
  recipients: DamageRecipient[];
  /** the damage to divide: the sum of the answer must be between these */
  totalMin: number;
  totalMax: number;
  /** damage left unassigned goes on to the player or planeswalker (trample) */
  trample: boolean;
  /** the server's defaults, already lethal-first */
  defaults: number[];
}

const OBJECT_ID = /object_id='([^']+)'/;

function objectId(html: string | undefined): string | null {
  return OBJECT_ID.exec(html ?? '')?.[1] ?? null;
}

/** "Grizzly Bears [1a2], P/T: 2/2" -> "Grizzly Bears" */
function creatureName(html: string | undefined): string {
  return stripMarkup(html).replace(/,\s*P\/T:.*$/i, '').replace(/\s*\[[0-9a-f]{3}\]/gi, '').trim();
}

/** Every permanent on the battlefield, by object id. */
function permanentsById(view: GameView | null | undefined): Map<string, PermanentView> {
  const map = new Map<string, PermanentView>();
  for (const player of view?.players ?? []) {
    for (const [id, permanent] of Object.entries(player.battlefield ?? {})) map.set(permanent.id ?? id, permanent);
  }
  return map;
}

/**
 * The server's ability icons cover deathtouch, granted or printed; reading them here keeps the rules glossary
 * (keywords.ts) out of the interaction model, which loads with every screen.
 */
function hasDeathtouch(permanent: PermanentView | undefined): boolean {
  return !!permanent && ((permanent.cardIcons ?? []) as { cardIconType?: string }[])
    .some((icon) => icon.cardIconType === 'ABILITY_DEATHTOUCH');
}

/** Damage that destroys this creature now: what its toughness has left, or a single point from a deathtouch source. */
export function lethalDamage(permanent: Pick<PermanentView, 'toughness' | 'damage'>, deathtouch = false): number {
  const toughness = Number(permanent.toughness);
  if (Number.isNaN(toughness)) return 0;
  const left = Math.max(0, toughness - (permanent.damage ?? 0));
  return deathtouch ? Math.min(1, left) : left;
}

/** Whether a multi-amount prompt is a combat damage assignment the board can draw. */
export function isCombatDamagePrompt(prompt: MultiAmountPrompt): boolean {
  return /combat damage/i.test(prompt.title ?? '') && prompt.items.length > 0 && prompt.items.every((item) => !!objectId(item.message));
}

/** The assignment behind a combat damage prompt, or null for any other multi-amount prompt. */
export function parseDamageAssignment(prompt: MultiAmountPrompt, view: GameView | null | undefined): DamageAssignment | null {
  if (!isCombatDamagePrompt(prompt)) return null;
  const permanents = permanentsById(view);
  const sourceId = objectId(prompt.header ?? undefined);
  const source = sourceId ? permanents.get(sourceId) : undefined;
  const deathtouch = hasDeathtouch(source);
  const recipients = prompt.items.map((item): DamageRecipient => {
    const id = objectId(item.message)!;
    const permanent = permanents.get(id);
    const min = item.min ?? 0;
    const max = item.max ?? Number.MAX_SAFE_INTEGER;
    return {
      id,
      name: permanent?.name ?? creatureName(item.message),
      // without the creature in view, the server's default is the best guess at lethal
      lethal: permanent ? lethalDamage(permanent, deathtouch) : (item.defaultValue ?? min),
      min,
      max,
    };
  });
  const header = prompt.header ?? '';
  return {
    sourceId,
    sourceName: source?.name ?? creatureName(/(?:blocking|blocked by)\s+(.*)$/i.exec(header)?.[1] ?? ''),
    recipients,
    totalMin: prompt.min,
    totalMax: prompt.max,
    trample: /trample/i.test(prompt.title ?? '') || prompt.min < prompt.max,
    defaults: prompt.items.map((item) => item.defaultValue ?? item.min ?? 0),
  };
}

type Limits = Pick<DamageAssignment, 'recipients' | 'totalMin' | 'totalMax'>;

/**
 * Lethal damage to each creature in order before the next one gets any (Arena's default). Damage the rules require
 * to be assigned (no trample) but left over after everything has lethal goes to the first creature that can take it;
 * with trample it stays unassigned and carries over to the player.
 */
export function lethalFirst({ recipients, totalMin, totalMax }: Limits): number[] {
  const values = recipients.map((recipient) => recipient.min);
  let left = totalMax - values.reduce((sum, value) => sum + value, 0);
  recipients.forEach((recipient, index) => {
    const add = Math.max(0, Math.min(recipient.lethal - values[index], recipient.max - values[index], left));
    values[index] += add;
    left -= add;
  });
  let short = totalMin - values.reduce((sum, value) => sum + value, 0);
  for (let index = 0; index < recipients.length && short > 0; index++) {
    const add = Math.min(short, recipients[index].max - values[index]);
    values[index] += add;
    short -= add;
  }
  return values;
}

export interface AssignmentCheck {
  valid: boolean;
  assigned: number;
  /** damage not yet on any creature: still to assign (no trample) or trampling over */
  unassigned: number;
  /** damage that must still be put on creatures before the answer is legal */
  missing: number;
}

export function checkAssignment(values: readonly number[], { recipients, totalMin, totalMax }: Limits): AssignmentCheck {
  const assigned = values.reduce((sum, value) => sum + value, 0);
  const inRange = values.length === recipients.length
    && recipients.every((recipient, index) => values[index] >= recipient.min && values[index] <= recipient.max);
  return {
    valid: inRange && assigned >= totalMin && assigned <= totalMax,
    assigned,
    unassigned: Math.max(0, totalMax - assigned),
    missing: Math.max(0, totalMin - assigned),
  };
}

/**
 * One more point of damage on a creature. When all the damage is already out, the point moves over from the last
 * other creature that has some to spare, so clicking a creature is enough to shift damage onto it.
 */
export function addDamage(values: readonly number[], index: number, limits: Limits): number[] {
  const { recipients, totalMax } = limits;
  const next = [...values];
  if (next[index] >= recipients[index].max) return next;
  const assigned = next.reduce((sum, value) => sum + value, 0);
  if (assigned < totalMax) {
    next[index] += 1;
    return next;
  }
  for (let other = next.length - 1; other >= 0; other--) {
    if (other !== index && next[other] > recipients[other].min) {
      next[other] -= 1;
      next[index] += 1;
      return next;
    }
  }
  return next;
}

/** One point less on a creature; it becomes unassigned (with trample, it carries over to the player). */
export function removeDamage(values: readonly number[], index: number, limits: Limits): number[] {
  const next = [...values];
  if (next[index] > limits.recipients[index].min) next[index] -= 1;
  return next;
}

/** The server's answer format: the amounts in prompt order, separated by spaces. */
export function assignmentAnswer(values: readonly number[]): string {
  return values.join(' ');
}

/**
 * The split to submit without asking, or null when the player has something to decide. Without trample, only when
 * the damage is exactly enough for every blocker to get lethal damage: nothing is left over and no blocker is spared.
 * With trample (and `trample` allowed), when every blocker gets lethal damage and the rest goes on to the player.
 */
export function autoAssignment(assignment: DamageAssignment, { trample }: { trample: boolean }): string | null {
  const { recipients, totalMin, totalMax } = assignment;
  if (recipients.length === 0 || recipients.some((recipient) => recipient.lethal <= 0)) return null;
  const values = lethalFirst(assignment);
  if (!checkAssignment(values, assignment).valid) return null;
  const allLethal = recipients.every((recipient, index) => values[index] === recipient.lethal);
  if (!allLethal) return null;
  const assigned = values.reduce((sum, value) => sum + value, 0);
  if (assignment.trample) return trample ? assignmentAnswer(values) : null;
  return assigned === totalMax && assigned >= totalMin ? assignmentAnswer(values) : null;
}

import type { CardView, GameView, PermanentView } from '../../protocol/generated/views';
import type { Command } from './interaction';
import type { Prompt } from './prompt';

/**
 * Choices the client makes, or starts, for the player when nothing is really being decided: a menu of one option, a
 * pick among interchangeable objects, and the obvious answers to proliferate.
 */

/** The answer to a menu with a single option that must be taken, or null. */
export function singleChoice(prompt: Prompt | null | undefined): Command | null {
  if (prompt?.kind === 'chooseAbility' && prompt.choices.length === 1) return { type: 'uuid', id: prompt.choices[0].id };
  if (prompt?.kind === 'chooseChoice' && prompt.choice.required) {
    const keyed = Object.keys(prompt.choice.keyChoices ?? {});
    const values = keyed.length > 0 ? keyed : prompt.choice.choices ?? [];
    if (values.length === 1) return { type: 'string', value: values[0] };
  }
  return null;
}

/** Permanents that differ in nothing a player would weigh (two untapped Treasure tokens) share a kind. */
export function permanentKind(permanent: PermanentView, controllerId: string | undefined): string {
  return JSON.stringify([
    'permanent',
    controllerId ?? null,
    permanent.name,
    permanent.power ?? null,
    permanent.toughness ?? null,
    !!permanent.tapped,
    !!permanent.summoningSickness,
    permanent.damage ?? 0,
    (permanent.counters ?? []).map((counter) => [counter.name, counter.count]),
    (permanent.attachments ?? []).length,
    permanent.rules ?? [],
  ]);
}

/** Cards off the battlefield are told apart by name and printing. */
export function cardKind(card: CardView): string {
  return JSON.stringify(['card', card.name, card.expansionSetCode ?? null]);
}

/** The kind of each object a prompt can point at, by id; players and stack objects have none. */
function kinds(prompt: Extract<Prompt, { kind: 'target' }>, view: GameView | null | undefined): Map<string, string> {
  const found = new Map<string, string>();
  for (const player of view?.players ?? []) {
    for (const [id, permanent] of Object.entries(player.battlefield ?? {})) found.set(id, permanentKind(permanent, player.playerId));
    for (const zone of [player.graveyard, player.exile]) {
      for (const [id, card] of Object.entries(zone ?? {})) found.set(id, cardKind(card));
    }
  }
  for (const [id, card] of Object.entries(view?.myHand ?? {})) found.set(id, cardKind(card));
  for (const [id, card] of Object.entries(prompt.cards ?? {})) found.set(id, cardKind(card));
  return found;
}

/**
 * The object to choose next when every object still to choose from is the same (sacrifice one of three Treasures,
 * discard from a hand of copies, fetch one of several basic Forests), or null when there's a real choice: different
 * objects, a player or a spell among them, an optional choice, or enough already chosen to stop.
 */
export function identicalPick(prompt: Prompt | null | undefined, view: GameView | null | undefined): string | null {
  if (prompt?.kind !== 'target' || !prompt.required) return null;
  if (prompt.chosen.length > 0 && prompt.doneLabel) return null;
  const open = prompt.targets.filter((id) => !prompt.chosen.includes(id));
  if (open.length === 0) return null;
  const known = kinds(prompt, view);
  const first = known.get(open[0]);
  if (!first || open.some((id) => known.get(id) !== first)) return null;
  return open[0];
}

/** Counters worth more of on your own things, and worth more of on an opponent's. */
const GOOD_COUNTERS = new Set([
  '+1/+1', 'loyalty', 'lore', 'charge', 'oil', 'quest', 'level', 'experience', 'energy', 'shield', 'flying', 'first strike',
  'double strike', 'lifelink', 'deathtouch', 'trample', 'vigilance', 'menace', 'hexproof', 'indestructible', 'reach',
]);
const BAD_COUNTERS = new Set(['-1/-1', 'poison', 'stun']);

function wanted(counters: { name?: string; count?: number }[] | undefined, mine: boolean): boolean {
  const present = (counters ?? []).filter((counter) => (counter.count ?? 0) > 0).map((counter) => (counter.name ?? '').toLowerCase());
  return present.length > 0 && present.every((name) => (mine ? GOOD_COUNTERS : BAD_COUNTERS).has(name));
}

/** Whether a prompt is proliferate's "choose any number of permanents and/or players with counters". */
export function isProliferate(prompt: Prompt | null | undefined): prompt is Extract<Prompt, { kind: 'target' }> {
  return prompt?.kind === 'target' && !prompt.required && /with counters on them/i.test(prompt.text);
}

/**
 * The obvious answers to proliferate: your own permanents and yourself when every counter on them is a good one, and
 * opponents and their permanents when every counter on them is a bad one (poison, -1/-1). Mixed ones stay unticked.
 */
export function proliferatePicks(prompt: Prompt | null | undefined, view: GameView | null | undefined, myId: string | null): string[] {
  if (!isProliferate(prompt) || !myId) return [];
  const picks = new Set<string>();
  for (const player of view?.players ?? []) {
    const mine = player.playerId === myId;
    if (player.playerId && wanted(player.counters, mine)) picks.add(player.playerId);
    for (const [id, permanent] of Object.entries(player.battlefield ?? {})) {
      // a permanent another player controls in my seat's battlefield is still judged by its controller
      const owner = permanent.controllerId ?? player.playerId;
      if (wanted(permanent.counters, owner === myId)) picks.add(id);
    }
  }
  return prompt.targets.filter((id) => picks.has(id));
}

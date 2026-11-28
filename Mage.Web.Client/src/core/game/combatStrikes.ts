import type { CardView, CombatGroupView, GameEventView, PermanentView } from '../../protocol/generated/views';
import { fighterOf, type Fighter } from './combatPreview';

/**
 * Combat strikes (#413): who hits whom in one update, how hard, and with which keywords. The match draws one lunge per
 * strike, so this is what decides that a double striker swings twice, that each of several blockers is hit and
 * strikes back, that trample goes through to the player, and that an attack on a planeswalker lands on the
 * planeswalker, even when the planeswalker died from it and the view's combat already names its player instead.
 *
 * Strikes come from the combat DAMAGE events when the server sends them; servers without events get them from the
 * combat groups and the step (an attacker's power at its first blocker, or at whom it attacks).
 */

export type HitWeight = 'light' | 'medium' | 'heavy' | 'huge';

export interface Strike {
  /** the creature striking */
  from: string;
  /** what it hit: a creature, planeswalker, battle or player */
  to: string;
  /** damage dealt (from the event), or the striker's power without events */
  amount: number;
  weight: HitWeight;
  /** an attacker striking, or a blocker striking the attacker it blocks */
  role: 'attack' | 'block';
  /** 0 for the first strike damage step (or the only one), 1 for regular damage after it in the same update */
  phase: number;
  /** stagger within the phase: one per attacker, and its blockers strike with it */
  order: number;
  /** trample: the blocker this strike went through on its way to the defender */
  through: string | null;
  firstStrike: boolean;
  doubleStrike: boolean;
  deathtouch: boolean;
  lifelink: boolean;
  infect: boolean;
  /** who controls the striker (a lifelink stream goes to them) */
  controllerId: string | null;
  /** guessed from the combat groups (no events): the amount is the striker's power, not damage dealt */
  estimated?: boolean;
}

/** How hard a hit lands, from the damage it deals. */
export function hitWeight(amount: number): HitWeight {
  if (amount >= 8) return 'huge';
  if (amount >= 5) return 'heavy';
  if (amount >= 3) return 'medium';
  return 'light';
}

export const WEIGHT_RANK: Record<HitWeight, number> = { light: 0, medium: 1, heavy: 2, huge: 3 };

/** The combat of one view, as the strike planner reads it. */
export interface CombatSide {
  /** attacker id to whom it attacks (a player, planeswalker or battle) */
  defenders: Map<string, string>;
  /** attacker id to the blockers blocking it, in order */
  blockers: Map<string, string[]>;
  /** blocker id to the attackers it blocks */
  blocking: Map<string, string[]>;
  /** combat cards by id, for their keywords when they've left the battlefield */
  cards: Map<string, CardView>;
  /** attackers that were blocked, whether or not their blockers are still there */
  blocked: Set<string>;
}

export function combatSide(groups: readonly CombatGroupView[] | undefined): CombatSide {
  const side: CombatSide = { defenders: new Map(), blockers: new Map(), blocking: new Map(), cards: new Map(), blocked: new Set() };
  for (const group of groups ?? []) {
    const blockers = Object.keys(group.blockers ?? {});
    for (const [id, card] of Object.entries(group.blockers ?? {})) side.cards.set(id, card);
    for (const [attacker, card] of Object.entries(group.attackers ?? {})) {
      side.cards.set(attacker, card);
      if (group.isBlocked || blockers.length > 0) side.blocked.add(attacker);
      if (group.defenderId) side.defenders.set(attacker, group.defenderId);
      // several views' groups add up: a view drops the blockers that died, the one before still has them
      const known = side.blockers.get(attacker) ?? [];
      side.blockers.set(attacker, [...known, ...blockers.filter((id) => !known.includes(id))]);
      for (const blocker of blockers) {
        const blocked = side.blocking.get(blocker) ?? [];
        if (!blocked.includes(attacker)) side.blocking.set(blocker, [...blocked, attacker]);
      }
    }
  }
  return side;
}

/**
 * Whom an attacker attacks, as the earlier view knew it when the later one fell back to a player: the Rust host names
 * the defending player once the attacked planeswalker or battle has left the battlefield (an attacker turned from a
 * planeswalker onto its player while attackers are declared keeps its new target: the planeswalker is still there).
 */
export function stableDefender(attacker: string, now: CombatSide, before: CombatSide | null, players: ReadonlySet<string>, present: ReadonlySet<string>): string | undefined {
  const current = now.defenders.get(attacker);
  const earlier = before?.defenders.get(attacker);
  if (earlier && current && current !== earlier && players.has(current) && !players.has(earlier) && !present.has(earlier)) return earlier;
  return current ?? earlier;
}

/**
 * The combat groups with each attacker still pointed at the planeswalker or battle it attacked after that left the
 * battlefield (see stableDefender), so the attack arrows don't swing over to the player. `previous` is what this
 * returned for the last view.
 */
export function keepDefenders(
  combat: readonly CombatGroupView[] | undefined,
  previous: readonly CombatGroupView[] | undefined,
  players: ReadonlySet<string>,
  present: ReadonlySet<string>,
): CombatGroupView[] | undefined {
  if (!combat || !previous || previous.length === 0) return combat as CombatGroupView[] | undefined;
  const now = combatSide(combat);
  const before = combatSide(previous);
  let changed = false;
  const kept = combat.map((group) => {
    const attackers = Object.keys(group.attackers ?? {});
    const defender = attackers.length > 0 ? stableDefender(attackers[0], now, before, players, present) : group.defenderId;
    if (!defender || defender === group.defenderId) return group;
    changed = true;
    return { ...group, defenderId: defender };
  });
  return changed ? kept : (combat as CombatGroupView[]);
}

type Lookup = (id: string) => CardView | PermanentView | undefined;

function fighter(lookup: Lookup, id: string): Fighter | null {
  const card = lookup(id);
  return card ? fighterOf(card, id) : null;
}

function strike(from: string, to: string, amount: number, role: Strike['role'], phase: number, order: number, through: string | null, who: Fighter | null): Strike {
  return {
    from, to, amount, weight: hitWeight(amount), role, phase, order, through,
    firstStrike: !!who?.first && !who.double,
    doubleStrike: !!who?.double,
    deathtouch: !!who?.deathtouch,
    lifelink: !!who?.lifelink,
    infect: !!who?.infect,
    controllerId: who?.controllerId ?? null,
  };
}

/**
 * Strikes from this update's combat DAMAGE events. `combat` is the combat as the views show it (this one's, falling
 * back to the last one's for creatures that died); `lookup` finds a creature's card for its keywords.
 */
export function strikesFromEvents(events: readonly GameEventView[], combat: CombatSide, lookup: Lookup): Strike[] {
  const hits = events
    .filter((event) => event.kind === 'DAMAGE' && event.combat && (event.amount ?? 0) > 0 && event.sourceId && event.targetId)
    .sort((a, b) => (a.seq ?? 0) - (b.seq ?? 0));
  if (hits.length === 0) return [];
  // first strike damage comes before regular damage: the first hit by a creature without first strike starts the
  // regular step. With only first and double strikers, a double striker hitting the same thing again does.
  const fighters = new Map<string, Fighter | null>();
  const who = (id: string) => {
    if (!fighters.has(id)) fighters.set(id, fighter(lookup, id));
    return fighters.get(id) ?? null;
  };
  let boundary = hits.findIndex((event) => !who(event.sourceId!)?.first);
  if (boundary <= 0) {
    const seen = new Set<string>();
    boundary = hits.findIndex((event) => {
      const key = `${event.sourceId}>${event.targetId}`;
      if (seen.has(key)) return true;
      seen.add(key);
      return false;
    });
  }
  const phaseOf = (index: number) => (boundary > 0 && index >= boundary ? 1 : 0);
  // one stagger slot per attacker, in the order they first hit (or were hit)
  const slots = new Map<string, number>();
  const slot = (attacker: string) => {
    if (!slots.has(attacker)) slots.set(attacker, slots.size);
    return slots.get(attacker)!;
  };
  const strikes: Strike[] = [];
  hits.forEach((event, index) => {
    const from = event.sourceId!;
    const to = event.targetId!;
    const phase = phaseOf(index);
    if (combat.defenders.has(from)) {
      const blockers = combat.blockers.get(from) ?? [];
      // trample: damage to the defender from an attacker that also hit a blocker in this step
      const through = blockers.includes(to)
        ? null
        : blockers.find((blocker) => hits.some((other, j) => phaseOf(j) === phase && other.sourceId === from && other.targetId === blocker)) ?? null;
      strikes.push(strike(from, to, event.amount!, 'attack', phase, slot(from), through, who(from)));
    } else if (combat.blocking.has(from)) {
      strikes.push(strike(from, to, event.amount!, 'block', phase, slot(combat.blocking.get(from)!.includes(to) ? to : combat.blocking.get(from)![0]), null, who(from)));
    } else {
      // a creature outside the combat the views showed (it left already): still a hit, not a lunge we can aim
      strikes.push(strike(from, to, event.amount!, 'attack', phase, slot(from), null, who(from)));
    }
  });
  // regular damage alone in this update is the update's first round
  if (strikes.every((item) => item.phase === 1)) for (const item of strikes) item.phase = 0;
  return strikes;
}

/**
 * Strikes for servers without events, from the combat groups when a combat damage step begins: each attacker hits each
 * of its blockers (or whom it attacks) for its power, and each blocker strikes back. First strikers swing in the first
 * damage step only, double strikers in both, the others in the regular one.
 */
export function strikesFromCombat(
  combat: CombatSide,
  step: string | undefined,
  beforeStep: string | undefined,
  lookup: Lookup,
  defenderOf: (attacker: string) => string | undefined,
): Strike[] {
  const isDamage = (value: string | undefined) => value === 'COMBAT_DAMAGE' || value === 'FIRST_COMBAT_DAMAGE';
  if (!isDamage(step) || step === beforeStep) return [];
  const swings = (who: Fighter | null) => {
    if (step === 'FIRST_COMBAT_DAMAGE') return !!who?.first;
    // the regular step after a first strike step: first strikers are done, double strikers swing again
    if (beforeStep === 'FIRST_COMBAT_DAMAGE') return !who?.first || !!who.double;
    return true;
  };
  const strikes: Strike[] = [];
  let order = 0;
  for (const [attacker, blockers] of combat.blockers) {
    const who = fighter(lookup, attacker);
    const slot = order++;
    // blocked, its blockers gone: it deals no damage, unless it tramples over to whom it attacks
    const stopped = blockers.length === 0 && combat.blocked.has(attacker) && !who?.trample;
    if (swings(who) && !stopped) {
      const targets = blockers.length > 0 ? blockers : [defenderOf(attacker)].filter((id): id is string => !!id);
      for (const target of targets) strikes.push({ ...strike(attacker, target, Math.max(1, who?.power ?? 1), 'attack', 0, slot, null, who), estimated: true });
    }
    for (const blocker of blockers) {
      const defender = fighter(lookup, blocker);
      if (swings(defender) && (defender?.power ?? 0) > 0) strikes.push({ ...strike(blocker, attacker, defender!.power, 'block', 0, slot, null, defender), estimated: true });
    }
  }
  return strikes;
}

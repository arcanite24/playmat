import type { CombatGroupView } from '../../protocol/generated/views';
import { fighterOf } from '../../core/game/combatPreview';
import type { PlayerBoard } from './boardModel';

/**
 * How attack arrows are drawn when many creatures attack: one arrow per stack on the table (identical tokens share a
 * stack), and once several stacks attack the same player or planeswalker, short feeders that join into one trunk
 * arrow with a count. A horde of tokens then draws one arrow across the opponent's board instead of a wall of them.
 */

/** this many attacking stacks or more, aimed at one defender, join into a trunk */
export const TRUNK_MIN_STACKS = 4;
/** where the feeders meet: this far (stage px) past the attackers nearest the defender, so they stay on their side */
const TRUNK_JOIN = 90;

export interface AttackSource {
  /** the stack's lead permanent, whose position the arrow leaves from */
  id: string;
  /** attacking creatures in this stack */
  count: number;
}

export interface AttackBundle {
  defender: string;
  sources: AttackSource[];
  /** attacking creatures aimed at the defender */
  count: number;
}

export interface Point {
  x: number;
  y: number;
}

/** Each permanent's stack lead, for every permanent drawn in a stack (lone permanents lead themselves). */
export function stackLeads(boards: readonly (PlayerBoard | null | undefined)[]): Map<string, string> {
  const leads = new Map<string, string>();
  for (const board of boards) {
    if (!board) continue;
    for (const group of [...board.front, ...board.back]) {
      const lead = group.lead.id!;
      for (const member of group.members) leads.set(member.id!, lead);
    }
  }
  return leads;
}

/** Attacks (attacker -> defender) grouped by defender, then by stack, in the order they came. */
export function bundleAttacks(attacks: readonly (readonly [string, string])[], leads: ReadonlyMap<string, string>): AttackBundle[] {
  const bundles = new Map<string, AttackBundle>();
  for (const [attacker, defender] of attacks) {
    let bundle = bundles.get(defender);
    if (!bundle) {
      bundle = { defender, sources: [], count: 0 };
      bundles.set(defender, bundle);
    }
    const lead = leads.get(attacker) ?? attacker;
    const source = bundle.sources.find((candidate) => candidate.id === lead);
    if (source) source.count++;
    else bundle.sources.push({ id: lead, count: 1 });
    bundle.count++;
  }
  return [...bundles.values()];
}

/** Whether a bundle is drawn as feeders and one trunk instead of an arrow per stack. */
export function isTrunk(bundle: Pick<AttackBundle, 'sources'>): boolean {
  return bundle.sources.length >= TRUNK_MIN_STACKS;
}

/**
 * Where a trunk's feeders meet: above (or below) the middle of the attackers, just past the row nearest the defender,
 * so the feeders stay on the attacker's side of the table and only the trunk crosses the defender's board.
 */
export function trunkJoin(sources: readonly Point[], defender: Point): Point {
  const x = sources.reduce((sum, point) => sum + point.x, 0) / sources.length;
  const centre = sources.reduce((sum, point) => sum + point.y, 0) / sources.length;
  const up = defender.y < centre;
  const edge = up ? Math.min(...sources.map((point) => point.y)) : Math.max(...sources.map((point) => point.y));
  const y = up ? Math.max(defender.y, edge - TRUNK_JOIN) : Math.min(defender.y, edge + TRUNK_JOIN);
  return { x, y };
}

/**
 * Attacking and blocking permanents, and the attack arrows, from the combat groups. Every arrow leaves an attacker for
 * what it will swing at when damage is dealt, so the lunge always follows its arrow: an unblocked attacker points at
 * the player, planeswalker or battle it attacks, a blocked one at each of its blockers. A blocked attacker whose
 * blockers have all left deals no damage and has no arrow, unless it tramples: then it points at whom it attacks.
 */
export function combatArrows(combat: readonly CombatGroupView[] | undefined) {
  const attacking = new Set<string>();
  const blocking = new Set<string>();
  const attacks: [string, string][] = [];
  for (const group of combat ?? []) {
    const blockers = Object.keys(group.blockers ?? {});
    blockers.forEach((id) => blocking.add(id));
    for (const [attacker, card] of Object.entries(group.attackers ?? {})) {
      attacking.add(attacker);
      if (blockers.length > 0) blockers.forEach((blocker) => attacks.push([attacker, blocker]));
      else if (group.defenderId && (!group.isBlocked || fighterOf(card, attacker).trample)) attacks.push([attacker, group.defenderId]);
    }
  }
  return { attacking, blocking, attacks };
}

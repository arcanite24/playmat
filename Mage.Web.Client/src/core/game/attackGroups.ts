import type { GameView, PermanentView } from '../../protocol/generated/views';
import type { Prompt } from './prompt';
import { myAttackers } from './interaction';

/**
 * Attacking with many of the same creature (a board of tokens) without clicking each one: identical creatures that
 * can attack are grouped, and the player says how many of each attack. The client then clicks them one at a time,
 * as the server asks (useAttackGroups); nothing here touches the session.
 */

/** creatures the player can't tell apart at a glance: same name, power, toughness, and token or not */
export interface AttackGroup {
  key: string;
  name: string;
  power: string;
  toughness: string;
  isToken: boolean;
  /** the first of them, for its picture */
  lead: PermanentView;
  /** those that can still be declared */
  ready: string[];
  /** those already attacking */
  attacking: string[];
}

/** groups this small are clicked by hand as easily */
export const MIN_ATTACK_GROUP = 3;

function groupKey(permanent: PermanentView): string {
  return [permanent.name ?? '', permanent.power ?? '', permanent.toughness ?? '', permanent.isToken ? 'token' : 'card'].join('|');
}

/** My identical creatures that can attack or are attacking, in board order, for groups of at least `min`. */
export function attackGroups(view: GameView | null | undefined, prompt: Prompt | null | undefined, min = MIN_ATTACK_GROUP): AttackGroup[] {
  if (!view || prompt?.kind !== 'declareAttackers') return [];
  const me = view.players?.find((player) => player.playerId === view.myPlayerId);
  if (!me) return [];
  const ready = new Set(prompt.possibleAttackers);
  const attacking = new Set(myAttackers(view));
  const groups = new Map<string, AttackGroup>();
  for (const permanent of Object.values(me.battlefield ?? {})) {
    const id = permanent.id;
    if (!id || (!ready.has(id) && !attacking.has(id))) continue;
    const key = groupKey(permanent);
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        name: permanent.name ?? '',
        power: permanent.power ?? '',
        toughness: permanent.toughness ?? '',
        isToken: !!permanent.isToken,
        lead: permanent,
        ready: [],
        attacking: [],
      };
      groups.set(key, group);
    }
    if (attacking.has(id)) group.attacking.push(id);
    else group.ready.push(id);
  }
  return [...groups.values()].filter((group) => group.ready.length + group.attacking.length >= min);
}

/** How many of a group can attack at most. */
export function groupSize(group: AttackGroup): number {
  return group.ready.length + group.attacking.length;
}

/**
 * The next creature to click to bring the groups to the wanted counts: one more to declare, or one to take back
 * (the last declared first). Creatures the server already refused are skipped; null when every group is where it
 * should be (or can't get closer).
 */
export function nextAttackClick(
  groups: readonly AttackGroup[],
  wanted: ReadonlyMap<string, number>,
  refused: ReadonlySet<string> = new Set(),
): string | null {
  for (const group of groups) {
    const want = wanted.get(group.key);
    if (want === undefined) continue;
    if (group.attacking.length < want) {
      const next = group.ready.find((id) => !refused.has(id));
      if (next) return next;
    } else if (group.attacking.length > want) {
      const next = [...group.attacking].reverse().find((id) => !refused.has(id));
      if (next) return next;
    }
  }
  return null;
}

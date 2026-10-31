import type { CardView, GameView, PermanentView } from '../../protocol/generated/views';
import { keywordMarks, mustBeBlocked } from './keywords';
import type { DeclareBlockersPrompt } from './prompt';

/**
 * Attackers that must be blocked if able (Fear of Being Hunted), and a check before blocks are sent.
 * The server is the judge: it refuses a declaration that breaks a requirement and asks again. This check only says
 * so before sending, from what the board shows (flying needs flying or reach; other evasion isn't known here), so a
 * second press sends the blocks anyway.
 */

function permanentOf(view: GameView, id: string): PermanentView | undefined {
  for (const player of view.players ?? []) {
    const permanent = player.battlefield?.[id];
    if (permanent) return permanent;
  }
  return undefined;
}

function canBlock(blocker: PermanentView | undefined, attacker: CardView): boolean {
  if (!blocker) return false;
  const attackerMarks = new Set(keywordMarks(attacker).map((mark) => mark.name));
  const blockerMarks = new Set(keywordMarks(blocker).map((mark) => mark.name));
  return !attackerMarks.has('Flying') || blockerMarks.has('Flying') || blockerMarks.has('Reach');
}

/**
 * The attacker the blocks as they stand leave unblocked although it must be blocked if able, or null: it is unblocked
 * while a creature that could block it blocks nothing or blocks an attacker without that need.
 */
export function blockWarning(view: GameView | null | undefined, prompt: DeclareBlockersPrompt): CardView | null {
  if (!view) return null;
  // in multiplayer only the attacks on this player (or on what they defend) are theirs to block
  const me = view.players?.find((player) => player.playerId === view.myPlayerId);
  const defendsHere = (defenderId: string | undefined) =>
    !defenderId || !me || defenderId === me.playerId || !!me.battlefield?.[defenderId];
  const blocking = new Map<string, string>(); // blocker -> the attacker it blocks
  for (const group of view.combat ?? []) {
    const first = Object.keys(group.attackers ?? {})[0];
    for (const id of Object.keys(group.blockers ?? {})) if (first) blocking.set(id, first);
  }
  for (const group of view.combat ?? []) {
    if (Object.keys(group.blockers ?? {}).length > 0 || !defendsHere(group.defenderId)) continue;
    for (const attacker of Object.values(group.attackers ?? {})) {
      if (!mustBeBlocked(attacker)) continue;
      const free = [...new Set([...prompt.possibleBlockers, ...blocking.keys()])].filter((id) => {
        const blocks = blocking.get(id);
        const other = blocks ? (view.combat ?? []).find((g) => g.attackers?.[blocks])?.attackers?.[blocks] : undefined;
        return (!other || !mustBeBlocked(other)) && canBlock(permanentOf(view, id), attacker);
      });
      if (free.length > 0) return attacker;
    }
  }
  return null;
}

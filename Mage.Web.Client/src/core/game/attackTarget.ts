import type { GameView } from '../../protocol/generated/views';
import type { DefenderOption } from './multiplayer';

/**
 * Whom an attacker goes at. Against one opponent with a planeswalker or battle out, XMage asks "which defender" for
 * every attacker; with Attack the player by default on, the opponent is the answer, and a creature dragged onto a
 * planeswalker or battle attacks that instead.
 */

/** The player to attack when the server asks, or null when there's a real choice of players (multiplayer). */
export function defaultDefender(options: readonly DefenderOption[]): string | null {
  const players = options.filter((option) => option.kind === 'player');
  return players.length === 1 ? players[0].id : null;
}

/** What an attacker can be dragged onto: the planeswalkers of other players and the battles on the board. */
export function attackDropTargets(view: GameView | null | undefined, myId: string | null): Set<string> {
  const targets = new Set<string>();
  for (const player of view?.players ?? []) {
    for (const [id, permanent] of Object.entries(player.battlefield ?? {})) {
      const types = permanent.cardTypes ?? [];
      if (types.includes('BATTLE') || (types.includes('PLANESWALKER') && player.playerId !== myId)) targets.add(id);
    }
  }
  return targets;
}

import type { LadderGame } from '../../../protocol/generated/views';
import { rankAt } from '../../career/rankedModel';
import styles from './Widgets.module.css';

/** the bot met most in the last ranked games, with the record against it */
export function rivalOf(games: readonly LadderGame[] | undefined): { name: string; deck: string; won: number; lost: number } | null {
  const met = new Map<string, { name: string; deck: string; won: number; lost: number; games: number }>();
  for (const game of games ?? []) {
    const id = game.opponent?.botId;
    if (!id || (game.status !== 'won' && game.status !== 'lost' && game.status !== 'drew')) continue;
    const entry = met.get(id) ?? { name: game.opponent?.name ?? id, deck: game.opponent?.deckName ?? '', won: 0, lost: 0, games: 0 };
    entry.games += 1;
    if (game.status === 'won') entry.won += 1;
    if (game.status === 'lost') entry.lost += 1;
    met.set(id, entry);
  }
  let best: { name: string; deck: string; won: number; lost: number; games: number } | null = null;
  for (const entry of met.values()) if (!best || entry.games > best.games) best = entry;
  return best ? { name: best.name, deck: best.deck, won: best.won, lost: best.lost } : null;
}

/** a ranked step's rank colour class */
export function rankClass(step: number): string {
  return styles[`rank_${rankAt(step).rank}`] ?? '';
}

import type { PaidGame } from '../../protocol/generated/views';

/** What others see of a player's progression: their level by their name, and the profile's progression part. */

/** The level worth a badge, or null: servers without Career, and players with no counted game, send none. */
export function shownLevel(level: number | undefined | null): number | null {
  return typeof level === 'number' && Number.isFinite(level) && level >= 1 ? Math.floor(level) : null;
}

/** How far into the current level (0..1), or null when the server didn't say how big the level is. */
export function levelShare(levelXp: number | undefined, levelSize: number | undefined): number | null {
  if (!levelSize || levelSize <= 0) return null;
  return Math.min(1, Math.max(0, (levelXp ?? 0) / levelSize));
}

export type GameKind = 'ai' | 'person' | 'pod' | 'event' | 'career' | 'other';

const KINDS: readonly GameKind[] = ['ai', 'person', 'pod', 'event', 'career'];

export function gameKind(kind: string | undefined): GameKind {
  return KINDS.find((known) => known === kind) ?? 'other';
}

/** The last few counted games, newest first. */
export function recentGames(recent: readonly PaidGame[] | undefined, count = 5): PaidGame[] {
  return [...(recent ?? [])].sort((a, b) => (b.at ?? 0) - (a.at ?? 0)).slice(0, count);
}

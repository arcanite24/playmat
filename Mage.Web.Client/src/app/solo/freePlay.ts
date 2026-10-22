import type { AdaptiveView, DeckCardLists as WireDeck, InboxNote } from '../../protocol/generated/views';
import type { MessageKey, Translate } from '../i18n';

/**
 * Free play against the computer (S045 to S049): where the AI's deck comes from, how the table is marked for a ghost
 * or for adaptive skill, and the inbox count. Pure, so the store and the screens share it and tests reach it.
 */

const GHOST_PREFIX = 'ghost:';
const MIRROR = 'mirror';

/**
 * The AI's deck, from the id kept in the AI options: a deck of the roster (saved, starter, format deck, or null for a
 * random pick), a ghost deck by its code, or a copy of the player's own deck.
 */
export type OpponentSource =
  | { kind: 'roster'; id: string | null }
  | { kind: 'ghost'; code: string }
  | { kind: 'mirror' };

export function opponentSource(id: string | null): OpponentSource {
  if (id === MIRROR) return { kind: 'mirror' };
  if (id?.startsWith(GHOST_PREFIX)) {
    const code = normalizeGhostCode(id.slice(GHOST_PREFIX.length));
    if (code) return { kind: 'ghost', code };
  }
  return { kind: 'roster', id: id && !id.startsWith(GHOST_PREFIX) ? id : null };
}

export const ghostOpponentId = (code: string) => `${GHOST_PREFIX}${normalizeGhostCode(code)}`;
export const MIRROR_OPPONENT_ID = MIRROR;

/** A ghost code as the server keeps it: no spaces or dashes, upper case. */
export function normalizeGhostCode(input: string): string {
  return input.trim().toUpperCase().replace(/[\s#-]/g, '');
}

/** The adaptive rating a free-play game moves: one for commander games, one for everything else. */
export function adaptiveFormatFor(commander: boolean): 'commander' | 'constructed' {
  return commander ? 'commander' : 'constructed';
}

/** The arguments of soloTableMark for a table, or null when the table is neither a ghost game nor adaptive. */
export function tableMarkArgs(source: OpponentSource, adaptiveFormat: string | null): [string | null, string | null] | null {
  const ghost = source.kind === 'ghost' ? source.code : null;
  return ghost || adaptiveFormat ? [ghost, adaptiveFormat] : null;
}

/** An adaptive skill as the server's AI takes it (1 to 10); a missing one plays at the default. */
export function adaptiveSkill(skill: number | undefined, fallback: number): number {
  return typeof skill === 'number' && Number.isFinite(skill) ? Math.max(1, Math.min(10, Math.round(skill))) : fallback;
}

/** True for a wire deck shaped like a commander deck: its commander (one or two) in the sideboard, and 99 or so cards. */
export function looksCommander(deck: WireDeck): boolean {
  const count = (cards: WireDeck['cards']) => (cards ?? []).reduce((sum, card) => sum + (card.amount ?? 0), 0);
  const side = count(deck.sideboard);
  return side >= 1 && side <= 2 && count(deck.cards) + side >= 98;
}


export function unseenCount(notes: readonly InboxNote[] | undefined): number {
  return (notes ?? []).filter((note) => !note.seen).length;
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 31_536_000_000], ['month', 2_592_000_000], ['week', 604_800_000], ['day', 86_400_000], ['hour', 3_600_000], ['minute', 60_000],
];

/** How long ago a moment was, as a value and a unit for Intl.RelativeTimeFormat (under a minute is 0 minutes). */
export function agoParts(at: number, now: number): { value: number; unit: Intl.RelativeTimeFormatUnit } {
  const elapsed = Math.max(0, now - at);
  for (const [unit, size] of UNITS) {
    if (elapsed >= size) return { value: -Math.floor(elapsed / size), unit };
  }
  return { value: 0, unit: 'minute' };
}

/** A ghost's record as "won of played", and its win share when it has played. */
export function ghostRecord(ghost: { won?: number; played?: number }): { won: number; played: number; percent: number | null } {
  const won = ghost.won ?? 0;
  const played = ghost.played ?? 0;
  return { won, played, percent: played > 0 ? Math.round((won / played) * 100) : null };
}

/** How hard adaptive skill plays, in words: the same bands as the server's sentence. */
export function adaptivePace(skill: number): 'gentle' | 'easy' | 'fair' | 'hard' | 'max' {
  if (skill <= 2) return 'gentle';
  if (skill <= 4) return 'easy';
  if (skill <= 6) return 'fair';
  if (skill <= 8) return 'hard';
  return 'max';
}

/** The adaptive skill as a sentence in the player's language; the number itself is never shown. */
export function adaptiveSentence(t: Translate, view: AdaptiveView | undefined): string | null {
  if (!view || view.skill == null) return null;
  const pace = t(`solo.adaptive.pace.${adaptivePace(view.skill)}` as MessageKey);
  if ((view.recentGames ?? 0) < 3) return t('solo.adaptive.sentenceNew', { pace });
  return t('solo.adaptive.sentence', { pace, wins: view.recentWins ?? 0, games: view.recentGames ?? 0 });
}

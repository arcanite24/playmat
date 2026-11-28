import { isCommanderFormat } from '../../core/decks/formats';
import type { LadderResult, LadderState } from '../../protocol/generated/views';

/**
 * Ranked's visible rank (#431), as the server counts it: a step from 0 (Bronze 4, no pips) up, four pips a tier,
 * four tiers a rank, Mythic at the top. These read a step for the screen; the server decides every move.
 */
export const RANKS = ['bronze', 'silver', 'gold', 'platinum', 'diamond', 'mythic'] as const;
export type RankName = (typeof RANKS)[number];

export const PIPS = 4;
export const TIERS = 4;
export const RANK_STEPS = PIPS * TIERS;
export const MYTHIC_STEP = RANK_STEPS * (RANKS.length - 1);

export interface RankAt {
  rank: RankName;
  /** 4 (lowest) to 1; 0 in Mythic */
  tier: number;
  pips: number;
}

export function rankAt(step: number): RankAt {
  const s = Math.max(0, Math.floor(step));
  if (s >= MYTHIC_STEP) return { rank: 'mythic', tier: 0, pips: 0 };
  const rank = RANKS[Math.floor(s / RANK_STEPS)];
  const within = s % RANK_STEPS;
  return { rank, tier: TIERS - Math.floor(within / PIPS), pips: within % PIPS };
}

/** the pips of the tier `step` is in, lit up to the step, and whether each was just won or just lost */
export type PipState = 'off' | 'on' | 'won' | 'lost';

export function pipRow(step: number, before?: number): PipState[] {
  const { rank, pips } = rankAt(step);
  if (rank === 'mythic') return [];
  const out: PipState[] = Array.from({ length: PIPS }, (_, index) => (index < pips ? 'on' : 'off'));
  if (before === undefined || before === step) return out;
  if (before < step) {
    // won pips: the ones lit by this game in this tier (a tier-up lights from the first)
    const sameTier = Math.floor(before / PIPS) === Math.floor(step / PIPS);
    const from = sameTier ? rankAt(before).pips : 0;
    for (let index = from; index < pips; index += 1) out[index] = 'won';
  } else if (Math.floor(before / PIPS) === Math.floor(step / PIPS)) {
    // a lost pip shows as it goes
    for (let index = pips; index < rankAt(before).pips; index += 1) out[index] = 'lost';
  }
  return out;
}

/** a step as words: "Gold 2", "Mythic" */
export function rankKey(rank: string | undefined): `career.ranked.rank.${RankName}` {
  const known = (RANKS as readonly string[]).includes(rank ?? '') ? (rank as RankName) : 'bronze';
  return `career.ranked.rank.${known}`;
}

/** how far up the ladder a step is, 0 to 1, for the climb bar */
export function climb(step: number): number {
  return Math.min(1, Math.max(0, step / MYTHIC_STEP));
}

/** the record as wins, losses and draws */
export function record(state: Pick<LadderState, 'won' | 'lost' | 'drew'> | undefined): { won: number; lost: number; drew: number; games: number } {
  const won = state?.won ?? 0;
  const lost = state?.lost ?? 0;
  const drew = state?.drew ?? 0;
  return { won, lost, drew, games: won + lost + drew };
}

/** what the result screen says moved: up a tier or rank, down a pip, held */
export type Movement = 'rankUp' | 'tierUp' | 'up' | 'held' | 'down' | 'tierDown';

export function movement(result: Pick<LadderResult, 'stepBefore' | 'stepAfter'>): Movement {
  const before = result.stepBefore ?? 0;
  const after = result.stepAfter ?? 0;
  if (after === before) return 'held';
  const a = rankAt(before);
  const b = rankAt(after);
  if (after > before) {
    if (a.rank !== b.rank) return 'rankUp';
    return a.tier !== b.tier ? 'tierUp' : 'up';
  }
  return a.tier !== b.tier || a.rank !== b.rank ? 'tierDown' : 'down';
}

/** the formats the screen offers, in the server's order, with the one in the address if it's among them */
export function pickFormat(formats: readonly string[] | undefined, wanted: string | null): string {
  const list = formats && formats.length ? formats : ['standard', 'pioneer'];
  return wanted && list.includes(wanted) ? wanted : list[0];
}

/** the ladder's formats the client knows the names of */
export type LadderFormat = 'standard' | 'pioneer' | 'modern' | 'commander' | 'pauper' | 'brawl' | 'legacy' | 'limited';

/** a format's name key: an unknown format reads as Standard */
export function formatKey(format: string | undefined): `career.ranked.format.${LadderFormat}` {
  const known: LadderFormat =
    format === 'pioneer' || format === 'modern' || format === 'commander' || format === 'pauper' || format === 'brawl' || format === 'legacy' || format === 'limited'
      ? format
      : 'standard';
  return `career.ranked.format.${known}`;
}

/** the localStorage key that says a season's end was shown for a format and queue */
export function seasonSeenKey(format: string, queue: string, season: number): string {
  return `playmat.seasonEnd.${queue}.${format}.${season}`;
}

/** server deckType string for a constructed ranked format (matches RankedService.deckType) */
export function rankedDeckType(format: string): string {
  if (format === 'pioneer') return 'Constructed - Pioneer';
  if (format === 'modern') return 'Constructed - Modern';
  if (format === 'commander') return 'Variant Magic - Commander';
  if (format === 'pauper') return 'Constructed - Pauper';
  if (format === 'brawl') return 'Variant Magic - Brawl';
  if (format === 'legacy') return 'Constructed - Legacy';
  return 'Constructed - Standard';
}

/** fast client pre-filter before asking the server validator */
export function isRankedCandidate(deck: { cardCount: number; format?: string }, format?: string): boolean {
  if (format === 'commander') return deck.cardCount >= 98 && isCommanderFormat(deck.format);
  if (format === 'brawl') {
    const low = (deck.format ?? '').toLowerCase();
    return deck.cardCount >= 58 && (low.includes('brawl') || isCommanderFormat(deck.format));
  }
  return deck.cardCount >= 60 && !isCommanderFormat(deck.format);
}

/** turns a cosmetic title id (e.g. "multicolor-champion" or "t-duelist") into readable words */
export function titleWords(id: string): string {
  const words = id.replace(/^t-/, '').replace(/-/g, ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

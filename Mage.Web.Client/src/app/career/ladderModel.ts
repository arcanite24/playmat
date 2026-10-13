import type {
  CareerHorde, CareerLimitedRun, CareerTwists, LimitedShape, MetaDeck, MetaFormat,
} from '../../protocol/generated/views';
import type { MessageKey } from '../i18n';
import { LIMITED_LOSSES, LIMITED_WINS, type Line } from './careerModesModel';

/**
 * The ladder past its first tiers and the modes beside it (S032–S050): difficulty pips, styles, boss phases, the weekly
 * challenge's rule, Limited run shapes and hordes, as lines and numbers the screens lay out.
 */

export const PIP_COUNT = 5;

/** five pips, the first {@code pips} lit; the measure is clamped to 1–5 */
export function pipSlots(pips: number | undefined): boolean[] {
  const lit = Math.max(1, Math.min(PIP_COUNT, Math.round(pips ?? 1)));
  return Array.from({ length: PIP_COUNT }, (_, index) => index < lit);
}

const STYLE_KEYS: Record<string, MessageKey> = {
  aggressive: 'career.style.aggressive',
  defensive: 'career.style.defensive',
  reckless: 'career.style.reckless',
  careful: 'career.style.careful',
  patient: 'career.style.patient',
};

/** an opponent's personality words as message keys; words the client doesn't know are left out */
export function styleKeys(style: readonly string[] | undefined): MessageKey[] {
  return (style ?? []).map((word) => STYLE_KEYS[word.toLowerCase()]).filter((key): key is MessageKey => !!key);
}

/** A boss's phases: what happens as its life falls (and any turn-timed twist), a line each, in order. */
export function phaseLines(twists: CareerTwists | undefined): Line[] {
  const timed = [...(twists?.timed ?? [])];
  const life = timed.filter((entry) => entry.when === 'life' && entry.life != null).sort((a, b) => (b.life ?? 0) - (a.life ?? 0));
  const turns = timed.filter((entry) => entry.when === 'turn' && entry.turn != null).sort((a, b) => (a.turn ?? 0) - (b.turn ?? 0));
  return [
    ...life.map((entry) => ({ key: 'career.phase.life' as const, vars: { life: entry.life ?? 0, what: entry.message ?? '' } })),
    ...turns.map((entry) => ({ key: 'career.phase.turn' as const, vars: { turn: entry.turn ?? 0, what: entry.message ?? '' } })),
  ].filter((line) => line.vars.what);
}

/** how the weekly challenge's board ranks a win */
export function challengeRuleKey(rule: string | undefined): MessageKey {
  if (rule === 'life') return 'career.challenge.ruleLife';
  if (rule === 'no-creatures') return 'career.challenge.ruleNoCreatures';
  return 'career.challenge.ruleTurns';
}

/** the board a weekly challenge's scores go to (older servers only had the one) */
export function challengeBoardMode(boardMode: string | undefined): string {
  return boardMode || 'weekly-challenge';
}

/** What ends a Limited run: wins and losses, or (a league) a number of games; 0 is "no limit". */
export interface RunLimits {
  maxWins: number;
  maxLosses: number;
  maxGames: number;
  league: boolean;
}

export function runLimits(run: Pick<CareerLimitedRun, 'shape' | 'maxWins' | 'maxLosses' | 'maxGames'>): RunLimits {
  const league = run.shape === 'league' || (!run.maxWins && !run.maxLosses && !!run.maxGames);
  if (league) return { maxWins: 0, maxLosses: 0, maxGames: run.maxGames || 5, league: true };
  return { maxWins: run.maxWins || LIMITED_WINS, maxLosses: run.maxLosses || LIMITED_LOSSES, maxGames: run.maxGames ?? 0, league: false };
}

/** what a shape pays by wins for a kind of run (a cube is a draft); the state's classic tables when the shape is unknown */
export function shapeRewards(
  shapes: readonly LimitedShape[] | undefined, shape: string | undefined, kind: string | undefined,
  fallback: { sealedRewards?: number[]; draftRewards?: number[] },
): number[] {
  const found = (shapes ?? []).find((item) => item.id === (shape || 'classic'));
  const source = found ?? fallback;
  return (kind === 'sealed' ? source.sealedRewards : source.draftRewards) ?? [];
}

/** A horde's rules, a line each. */
export function hordeLines(horde: Pick<CareerHorde, 'survive' | 'flip' | 'rampEvery' | 'life' | 'deckSize'>): Line[] {
  const lines: Line[] = [{ key: 'career.horde.survive', vars: { count: horde.survive ?? 0 } }];
  lines.push({ key: 'career.horde.flip', vars: { count: horde.flip ?? 1 } });
  if (horde.rampEvery) lines.push({ key: 'career.horde.ramp', vars: { count: horde.rampEvery } });
  if (horde.life != null && horde.life !== 20) lines.push({ key: 'career.horde.life', vars: { life: horde.life } });
  if (horde.deckSize) lines.push({ key: 'career.horde.deckSize', vars: { count: horde.deckSize } });
  return lines;
}

/** The meta library's 60-card constructed decks, by format; formats with none are left out. */
export function constructedFormats(formats: readonly MetaFormat[] | undefined): { id: string; name: string; decks: MetaDeck[] }[] {
  return (formats ?? [])
    .map((format) => ({
      id: format.id ?? '',
      name: format.name ?? format.id ?? '',
      decks: (format.decks ?? []).filter((deck) => !!deck.file && (deck.deckType ?? '').startsWith('Constructed')),
    }))
    .filter((format) => format.decks.length > 0);
}

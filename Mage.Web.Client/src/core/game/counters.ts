import type { PermanentView } from '../../protocol/generated/views';

/**
 * The counters on a permanent as badges for the board: a +1/+1 counter reads "+1/+1", the counters with a
 * symbol (stun, time, finality...) show that symbol and how many, and an impending permanent (an Overlord cast for
 * its impending cost) says how many time counters are left before it becomes a creature.
 */
export type CounterKind = 'pt' | 'time' | 'stun' | 'finality' | 'shield' | 'lore' | 'oil' | 'age' | 'fade' | 'ice' | 'charge' | 'nest' | 'other';

export interface CounterBadge {
  /** the counter's name as the server sends it */
  name: string;
  count: number;
  kind: CounterKind;
  /** text printed on the badge beside its symbol ("+1/+1", a short unknown name); empty when the symbol says it */
  label: string;
  /** the counter's own word for the tooltip ("+1/+1", "bounty"); the symbol kinds have their own help in the catalogue */
  word: string;
  /** the count is printed too (always for time counters: the number is the point) */
  showCount: boolean;
  /** an impending permanent's time counters: the badge reads "Impending N" */
  impending?: boolean;
}

const SYMBOLS: Record<string, CounterKind> = {
  time: 'time', stun: 'stun', finality: 'finality', shield: 'shield', lore: 'lore', oil: 'oil', age: 'age', fade: 'fade',
  ice: 'ice', charge: 'charge', nest: 'nest',
};

/** Cast for its impending cost: it has time counters, isn't a creature yet, and has impending. */
export function isImpending(permanent: Pick<PermanentView, 'counters' | 'cardTypes' | 'rules'>): boolean {
  const time = (permanent.counters ?? []).find((counter) => counter.name?.toLowerCase() === 'time');
  return !!time && (time.count ?? 0) > 0
    && !(permanent.cardTypes ?? []).includes('CREATURE')
    && (permanent.rules ?? []).some((line) => /\bimpending\b/i.test(line));
}

export function counterBadges(permanent: Pick<PermanentView, 'counters' | 'cardTypes' | 'rules'>): CounterBadge[] {
  const impending = isImpending(permanent);
  return (permanent.counters ?? []).filter((counter) => (counter.count ?? 0) > 0).map((counter) => {
    const name = counter.name ?? '';
    const count = counter.count ?? 0;
    if (/^[+-]\d+\/[+-]\d+$/.test(name)) {
      return { name, count, kind: 'pt' as const, label: name.replace(/-/g, '−'), word: name, showCount: count > 1 };
    }
    const key = name.replace(/\s*counters?$/i, '').trim().toLowerCase();
    const kind = SYMBOLS[key];
    if (kind === 'time' && impending) {
      return { name, count, kind, label: '', word: key, showCount: false, impending: true };
    }
    if (kind) return { name, count, kind, label: '', word: key, showCount: kind === 'time' || count > 1 };
    // a counter without a symbol keeps its name, whole when it is short
    const label = key.length <= 10 ? key : `${key.slice(0, 9)}…`;
    return { name, count, kind: 'other' as const, label, word: key, showCount: count > 1 };
  });
}

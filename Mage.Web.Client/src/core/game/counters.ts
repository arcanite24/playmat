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
  /** text printed on the badge beside its symbol ("+1/+1", "Impending 4"); empty when the symbol says it */
  label: string;
  /** the count is printed too (always for time counters: the number is the point) */
  showCount: boolean;
  title: string;
  impending?: boolean;
}

const SYMBOLS: Record<string, { kind: CounterKind; help: string }> = {
  time: { kind: 'time', help: 'time counter' },
  stun: { kind: 'stun', help: "stun counter: instead of untapping, it loses one" },
  finality: { kind: 'finality', help: 'finality counter: if it would die, it is exiled instead' },
  shield: { kind: 'shield', help: 'shield counter: damage or destruction removes the counter instead' },
  lore: { kind: 'lore', help: 'lore counter' },
  oil: { kind: 'oil', help: 'oil counter' },
  age: { kind: 'age', help: 'age counter' },
  fade: { kind: 'fade', help: 'fade counter' },
  ice: { kind: 'ice', help: 'ice counter' },
  charge: { kind: 'charge', help: 'charge counter' },
  nest: { kind: 'nest', help: 'nest counter' },
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
    const plural = count === 1 ? '' : 's';
    if (/^[+-]\d+\/[+-]\d+$/.test(name)) {
      return { name, count, kind: 'pt' as const, label: name.replace(/-/g, '−'), showCount: count > 1, title: `${count} ${name} counter${plural}` };
    }
    const key = name.replace(/\s*counters?$/i, '').trim().toLowerCase();
    const symbol = SYMBOLS[key];
    if (symbol?.kind === 'time' && impending) {
      return {
        name, count, kind: 'time' as const, label: `Impending ${count}`, showCount: false, impending: true,
        title: `Impending: ${count} time counter${plural} left. It isn't a creature until the last is removed; one comes off at each of its controller's end steps.`,
      };
    }
    if (symbol) {
      return { name, count, kind: symbol.kind, label: '', showCount: symbol.kind === 'time' || count > 1, title: `${count} ${symbol.help.replace('counter', `counter${plural}`)}` };
    }
    // a counter without a symbol keeps its name, whole when it is short
    const label = key.length <= 10 ? key : `${key.slice(0, 9)}…`;
    return { name, count, kind: 'other' as const, label, showCount: count > 1, title: `${count} ${key} counter${plural}` };
  });
}

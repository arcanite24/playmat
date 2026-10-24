import { Ban, BatteryCharging, BookOpen, Clock, Droplet, Egg, Ghost, Hourglass, Shield, Snowflake, Sparkles, type LucideIcon } from 'lucide-react';
import { useMemo } from 'react';
import { counterBadges, type CounterKind } from '../../core/game/counters';
import type { PermanentView } from '../../protocol/generated/views';
import styles from './CounterBadges.module.css';

/** The symbol each kind of counter shows; +1/+1 counters and unknown ones print their name instead. */
const SYMBOLS: Partial<Record<CounterKind, LucideIcon>> = {
  time: Hourglass,
  stun: Sparkles,
  finality: Ban,
  shield: Shield,
  lore: BookOpen,
  oil: Droplet,
  age: Clock,
  fade: Ghost,
  ice: Snowflake,
  charge: BatteryCharging,
  nest: Egg,
};

/** A permanent's counters on the board, as symbols with their count ("+1/+1", an hourglass and 4). */
export function CounterBadges({ permanent, className }: { permanent: PermanentView; className?: string }) {
  const badges = useMemo(() => counterBadges(permanent), [permanent]);
  if (badges.length === 0) return null;
  return (
    <ul className={className} aria-label="Counters">
      {badges.map((badge) => {
        const Icon = SYMBOLS[badge.kind];
        return (
          <li key={badge.name} className={badge.impending ? styles.impending : styles[badge.kind]} title={badge.title}>
            {Icon && <Icon className={styles.symbol} size={18} strokeWidth={2.4} aria-hidden="true" />}
            {badge.label && <span>{badge.label}</span>}
            {badge.showCount && <b>{badge.count}</b>}
          </li>
        );
      })}
    </ul>
  );
}

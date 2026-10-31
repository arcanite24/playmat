import { Ban, BatteryCharging, BookOpen, Clock, Droplet, Egg, Ghost, Hourglass, Shield, Snowflake, ZapOff, type LucideIcon } from 'lucide-react';
import { useMemo } from 'react';
import { counterBadges, type CounterBadge, type CounterKind } from '../../core/game/counters';
import type { PermanentView } from '../../protocol/generated/views';
import { useT, type MessageKey, type Translate } from '../i18n';
import styles from './CounterBadges.module.css';

/** The symbol each kind of counter shows; +1/+1 counters and unknown ones print their name instead. */
const SYMBOLS: Partial<Record<CounterKind, LucideIcon>> = {
  time: Hourglass,
  stun: ZapOff,
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

/** What the badge says when pointed at: the counter and, for the ones with a rule of their own, that rule. */
function titleOf(t: Translate, badge: CounterBadge): string {
  if (badge.impending) return t('match.counter.impendingHelp', { count: badge.count });
  if (badge.kind === 'pt' || badge.kind === 'other') return t('match.counter.named', { count: badge.count, name: badge.word });
  return t(`match.counter.${badge.kind}` as MessageKey, { count: badge.count });
}

/** A permanent's counters on the board, as symbols with their count ("+1/+1", an hourglass and 4). */
export function CounterBadges({ permanent, className }: { permanent: PermanentView; className?: string }) {
  const t = useT();
  const badges = useMemo(() => counterBadges(permanent), [permanent]);
  if (badges.length === 0) return null;
  return (
    <ul className={className} aria-label={t('match.counter.list')}>
      {badges.map((badge) => {
        const Icon = SYMBOLS[badge.kind];
        return (
          <li key={badge.name} className={badge.impending ? styles.impending : styles[badge.kind]} title={titleOf(t, badge)}>
            {Icon && <Icon className={styles.symbol} size={18} strokeWidth={2.4} aria-hidden="true" />}
            {badge.impending ? <span>{t('match.counter.impending', { count: badge.count })}</span> : badge.label && <span>{badge.label}</span>}
            {badge.showCount && <b>{badge.count}</b>}
          </li>
        );
      })}
    </ul>
  );
}

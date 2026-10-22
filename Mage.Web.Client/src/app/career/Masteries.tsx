import type { CareerMastery } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { fraction, masteryName } from './progressModel';
import styles from './Masteries.module.css';

/**
 * Masteries (B290): a track for each color and format, ten ranks from games played, with what the next rank takes.
 */
export function Masteries({ list }: { list: CareerMastery[] }) {
  const t = useT();
  const groups: { kind: string; label: MessageKey }[] = [
    { kind: 'color', label: 'career.mastery.colors' },
    { kind: 'format', label: 'career.mastery.formats' },
  ];
  return (
    <div className={styles.masteries}>
      <p className={styles.note}>{t('career.mastery.note')}</p>
      {groups.map((group) => (
        <section key={group.kind} aria-label={t(group.label)}>
          <h3 className={styles.head}>{t(group.label)}</h3>
          <ul className={styles.tracks}>
            {list.filter((mastery) => mastery.kind === group.kind).map((mastery) => {
              const top = (mastery.rank ?? 0) >= (mastery.maxRank ?? 10);
              const name = masteryName(t, mastery.track);
              return (
                <li key={mastery.track} className={[styles.track, top ? styles.top : ''].join(' ')}>
                  <span className={styles.name}>{name}</span>
                  <span className={styles.rank}>{t('career.mastery.rank', { rank: mastery.rank ?? 0, max: mastery.maxRank ?? 10 })}</span>
                  <span className={styles.pips} aria-hidden="true">
                    {Array.from({ length: mastery.maxRank ?? 10 }, (_, index) => <i key={index} className={index < (mastery.rank ?? 0) ? styles.on : ''} />)}
                  </span>
                  <span className={styles.next}>
                    {top ? t('career.mastery.top') : t('career.mastery.next', { games: mastery.games ?? 0, at: mastery.nextAt ?? 0 })}
                  </span>
                  {!top && <span className={styles.bar} aria-hidden="true"><i style={{ width: `${fraction(mastery.games, mastery.nextAt) * 100}%` }} /></span>}
                </li>
              );
            })}
          </ul>
        </section>
      ))}
    </div>
  );
}

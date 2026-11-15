import { Square, Swords } from 'lucide-react';
import { groupSize, type AttackGroup } from '../../core/game/attackGroups';
import { useT } from '../i18n';
import './matchMessages';
import styles from './AttackGroups.module.css';

const MAX_GROUPS = 5;

/**
 * While you declare attackers with a board of identical creatures (tokens, mostly): one row per kind with how many
 * of them attack, set with a slider or typed, and All / None. The clicks are made for you (useAttackGroups).
 */
export function AttackGroups({ groups, wanted, busy, remaining, onWant, onStop }: {
  groups: readonly AttackGroup[];
  wanted: ReadonlyMap<string, number>;
  busy: boolean;
  remaining: number;
  onWant(group: AttackGroup, count: number): void;
  onStop(): void;
}) {
  const t = useT();
  if (groups.length === 0) return null;
  return (
    <section className={styles.panel} aria-label={t('match.attackGroups.title')}>
      <h3 className={styles.title}><Swords size={18} aria-hidden="true" /> {t('match.attackGroups.title')}</h3>
      <ul className={styles.rows}>
        {groups.slice(0, MAX_GROUPS).map((group) => {
          const size = groupSize(group);
          const count = wanted.get(group.key) ?? group.attacking.length;
          const label = `${group.name} ${group.power}/${group.toughness}`;
          const set = (value: number) => onWant(group, Math.max(0, Math.min(size, Math.round(value))));
          return (
            <li key={group.key} className={styles.row}>
              <span className={styles.name}>
                {group.name} <small>{group.power}/{group.toughness}</small>
              </span>
              <input
                type="range"
                className={styles.slider}
                min={0}
                max={size}
                value={count}
                onChange={(event) => set(Number(event.target.value))}
                aria-label={t('match.attackGroups.how', { name: label })}
              />
              <span className={styles.count}>
                <input
                  type="number"
                  className={styles.number}
                  min={0}
                  max={size}
                  value={count}
                  onChange={(event) => { if (event.target.value !== '') set(Number(event.target.value)); }}
                  aria-label={t('match.attackGroups.how', { name: label })}
                />
                <span aria-hidden="true">/ {size}</span>
              </span>
              <button type="button" className={styles.quick} onClick={() => set(size)} disabled={count === size} aria-label={t('match.attackGroups.allOf', { name: label })}>
                {t('match.attackGroups.all')}
              </button>
              <button type="button" className={styles.quick} onClick={() => set(0)} disabled={count === 0} aria-label={t('match.attackGroups.noneOf', { name: label })}>
                {t('match.attackGroups.none')}
              </button>
            </li>
          );
        })}
      </ul>
      {busy && (
        <p className={styles.busy} aria-live="polite">
          {t('match.attackGroups.busy', { count: remaining })}
          <button type="button" className={styles.quick} onClick={onStop}><Square size={14} aria-hidden="true" /> {t('match.attackGroups.stop')}</button>
        </p>
      )}
    </section>
  );
}

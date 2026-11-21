import { Minus, Plus, Skull } from 'lucide-react';
import { useMemo } from 'react';
import { lethalFirst } from '../../core/game/damageAssignment';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import { useObjectBoxes } from './boardAnchors';
import type { DamageSplit } from './useDamageSplit';
import './matchMessages';
import styles from './DamageAssigner.module.css';

/** Damage marked on each creature it is assigned to, with steppers; clicking the creature adds a point. */
export function DamageAssigner({ split }: { split: DamageSplit }) {
  const t = useT();
  const { assignment, values, check } = split;
  const ids = useMemo(() => [...assignment.recipients.map((recipient) => recipient.id), ...(assignment.sourceId ? [assignment.sourceId] : [])], [assignment]);
  const boxes = useObjectBoxes(ids);
  const sourceBox = assignment.sourceId ? boxes.get(assignment.sourceId) : undefined;
  const changed = values.some((value, index) => value !== lethalFirst(assignment)[index]);

  return (
    <div className={styles.layer}>
      {assignment.recipients.map((recipient, index) => {
        const box = boxes.get(recipient.id);
        if (!box) return null;
        const value = values[index];
        const lethal = recipient.lethal > 0 && value >= recipient.lethal;
        return (
          <div
            key={recipient.id}
            className={[styles.chip, lethal ? styles.lethal : ''].join(' ')}
            style={{ left: box.x + box.width / 2, top: box.y + box.height / 2 }}
            role="group"
            aria-label={t(lethal ? 'match.split.chip.lethal' : 'match.split.chip', { name: recipient.name ?? '', count: value })}
          >
            <span className={styles.order} aria-hidden="true">{index + 1}</span>
            <button
              type="button"
              className={styles.step}
              onClick={() => split.remove(index)}
              disabled={value <= recipient.min}
              aria-label={t('match.split.less', { name: recipient.name ?? '' })}
            >
              <Minus size={20} strokeWidth={3} />
            </button>
            <span className={styles.amount}>
              <b>{value}</b>
              <small>{recipient.lethal > 0 ? t('match.split.of', { count: recipient.lethal }) : t('match.split.dead')}</small>
            </span>
            <button
              type="button"
              className={styles.step}
              onClick={() => split.add(index)}
              disabled={value >= recipient.max}
              aria-label={t('match.split.more', { name: recipient.name ?? '' })}
            >
              <Plus size={20} strokeWidth={3} />
            </button>
            {lethal && <Skull className={styles.skull} size={18} aria-hidden="true" />}
          </div>
        );
      })}
      {sourceBox && (
        <div className={styles.source} style={{ left: sourceBox.x + sourceBox.width / 2, top: sourceBox.y + sourceBox.height + 8 }}>
          <span className={styles.total}>
            {t('match.split.assigned', { count: check.assigned, max: assignment.totalMax })}
            {assignment.trample && check.unassigned > 0 && <> · {t('match.split.trample', { count: check.unassigned })}</>}
          </span>
          {changed && <Button variant="quiet" size="sm" onClick={split.reset}>{t('match.split.lethalFirst')}</Button>}
        </div>
      )}
    </div>
  );
}

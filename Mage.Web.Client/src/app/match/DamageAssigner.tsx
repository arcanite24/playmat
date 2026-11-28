import { Minus, Plus, Skull } from 'lucide-react';
import { useMemo } from 'react';
import { lethalFirst } from '../../core/game/damageAssignment';
import { Button } from '../ui/Button';
import { useObjectBoxes } from './boardAnchors';
import type { DamageSplit } from './useDamageSplit';
import styles from './DamageAssigner.module.css';

/** Damage marked on each creature it is assigned to, with steppers; clicking the creature adds a point. */
export function DamageAssigner({ split }: { split: DamageSplit }) {
  const { assignment, values, check } = split;
  const ids = useMemo(() => [...assignment.recipients.map((recipient) => recipient.id), ...(assignment.sourceId ? [assignment.sourceId] : [])], [assignment]);
  const boxes = useObjectBoxes(ids);
  const sourceBox = assignment.sourceId ? boxes.get(assignment.sourceId) : undefined;
  const changed = values.some((value, index) => value !== lethalFirst(assignment)[index]);

  return (
    <div className={styles.layer} data-pad>
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
            aria-label={`${recipient.name}: ${value} damage${lethal ? ', lethal' : ''}`}
          >
            <span className={styles.order} aria-hidden="true">{index + 1}</span>
            <button
              type="button"
              className={styles.step}
              onClick={() => split.remove(index)}
              disabled={value <= recipient.min}
              aria-label={`One less damage to ${recipient.name}`}
            >
              <Minus size={20} strokeWidth={3} />
            </button>
            <span className={styles.amount}>
              <b>{value}</b>
              <small>{recipient.lethal > 0 ? `of ${recipient.lethal}` : 'dead'}</small>
            </span>
            <button
              type="button"
              className={styles.step}
              onClick={() => split.add(index)}
              disabled={value >= recipient.max}
              aria-label={`One more damage to ${recipient.name}`}
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
            {check.assigned}/{assignment.totalMax} assigned
            {assignment.trample && check.unassigned > 0 && <> · {check.unassigned} trample</>}
          </span>
          {changed && <Button variant="quiet" size="sm" onClick={split.reset}>Lethal first</Button>}
        </div>
      )}
    </div>
  );
}

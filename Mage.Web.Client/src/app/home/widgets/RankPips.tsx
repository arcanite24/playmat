import { pipRow } from '../../career/rankedModel';
import styles from './Widgets.module.css';

/** a ranked step as its rank colour and the tier's pips */
export function RankPips({ step, label }: { step: number; label: string }) {
  const pips = pipRow(step);
  if (pips.length === 0) return null;
  return (
    <span className={styles.pips} role="img" aria-label={label}>
      {pips.map((pip, index) => <i key={index} className={[styles.pip, pip === 'on' ? styles.pipOn : ''].join(' ')} />)}
    </span>
  );
}

import styles from './ManaCurve.module.css';

/** Nonland cards by mana value, 0 to 7+, as printed bars. */
export function ManaCurve({ curve, className }: { curve: number[]; className?: string }) {
  const peak = Math.max(1, ...curve);
  const spoken = curve.map((value, index) => `${value} at ${index === 7 ? '7 or more' : index}`).join(', ');
  return (
    <div className={[styles.curve, className ?? ''].join(' ')} role="img" aria-label={`Mana curve: ${spoken}`}>
      {curve.map((value, index) => (
        <div key={index} className={styles.bar} title={`${value} ${index === 7 ? '7+' : index}-drops`}>
          <span className={styles.barTrack}>
            <span className={styles.barFill} style={{ transform: `scaleY(${value / peak})` }} />
            {value > 0 && <b className={styles.barValue} style={{ bottom: `calc(${(value / peak) * 100}% + 2px)` }}>{value}</b>}
          </span>
          <span className={styles.barLabel}>{index === 7 ? '7+' : index}</span>
        </div>
      ))}
    </div>
  );
}

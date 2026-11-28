import { Moon, Sun } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';
import './matchMessages';
import styles from './DayNightBadge.module.css';

/**
 * Day / Night HUD indicator (#407): shown once the game has entered day or night (e.g. Daybound/Nightbound or Vadrik),
 * pulses briefly on transition, and opens a rules hint popover explaining how day and night switch.
 */
export function DayNightBadge({ daytime }: { daytime: boolean | null | undefined }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const [tracked, setTracked] = useState<{ daytime: boolean | null | undefined; changes: number }>({ daytime, changes: 0 });
  if (tracked.daytime !== daytime) {
    const flipped = tracked.daytime !== undefined && tracked.daytime !== null
      && daytime !== undefined && daytime !== null
      && tracked.daytime !== daytime;
    setTracked({ daytime, changes: flipped ? tracked.changes + 1 : tracked.changes });
  }
  const changedKey = tracked.changes;

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false);
    };
    window.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('keydown', onKey);
    };
  }, [open]);

  if (daytime !== true && daytime !== false) return null;

  const isDay = daytime;
  const label = t(isDay ? 'match.dayNight.day' : 'match.dayNight.night');
  const shown = open || hovered;

  return (
    <div
      ref={wrapRef}
      className={styles.wrap}
      onPointerEnter={(event) => { if (event.pointerType !== 'touch') setHovered(true); }}
      onPointerLeave={() => setHovered(false)}
    >
      <button
        key={changedKey}
        type="button"
        className={[styles.badge, isDay ? styles.day : styles.night, changedKey > 0 ? styles.flip : ''].join(' ')}
        aria-expanded={open}
        aria-label={t(isDay ? 'match.dayNight.ariaDay' : 'match.dayNight.ariaNight')}
        onClick={(event) => {
          event.stopPropagation();
          setOpen((current) => !current);
        }}
      >
        {isDay ? <Sun size={18} aria-hidden="true" /> : <Moon size={18} aria-hidden="true" />}
        <span>{label}</span>
      </button>
      <span className={styles.srOnly} aria-live="polite">
        {changedKey > 0 ? t(isDay ? 'match.dayNight.nowDay' : 'match.dayNight.nowNight') : ''}
      </span>
      {shown && (
        <div className={styles.popover} role="tooltip">
          <p className={styles.popTitle}>
            {isDay ? <Sun size={16} aria-hidden="true" /> : <Moon size={16} aria-hidden="true" />}
            {t(isDay ? 'match.dayNight.stateDay' : 'match.dayNight.stateNight')}
          </p>
          <ul className={styles.rules}>
            <li>{t('match.dayNight.toNight')}</li>
            <li>{t('match.dayNight.toDay')}</li>
          </ul>
        </div>
      )}
    </div>
  );
}

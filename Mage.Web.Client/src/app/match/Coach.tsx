import { useEffect, useMemo, useRef } from 'react';
import { nextTip, type CoachTip } from '../../core/game/coach';
import type { InteractionMode } from '../../core/game/interaction';
import type { GameView } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { useCoach } from '../stores/coach';
import { Button } from '../ui/Button';
import './matchMessages';
import styles from './Coach.module.css';

/** each tip's title and text: match.coach.<tip>.title and .text */
const tipKeys = (tip: CoachTip) => ({ title: `match.coach.${tip}.title` as MessageKey, text: `match.coach.${tip}.text` as MessageKey });

/** A tip for each new kind of moment in the guided first game; never in the way of the choice itself. */
export function Coach({ view, mode, awaiting, gameOver }: { view: GameView | null; mode: InteractionMode; awaiting: boolean; gameOver: boolean }) {
  const t = useT();
  const guided = useCoach((state) => state.guided);
  const seen = useCoach((state) => state.seen);
  const markSeen = useCoach((state) => state.markSeen);
  const finish = useCoach((state) => state.finish);
  const seenSet = useMemo(() => new Set(seen), [seen]);
  const tip = guided && !gameOver ? nextTip({ view, mode, awaiting }, seenSet) : null;
  // a tip the player acted on has done its job: answering anything while it shows counts as reading it
  const shown = useRef<CoachTip | null>(null);
  useEffect(() => {
    if (tip) shown.current = tip;
    else if (awaiting && shown.current) {
      markSeen(shown.current);
      shown.current = null;
    }
  }, [tip, awaiting, markSeen]);

  useEffect(() => {
    if (guided && gameOver) finish();
  }, [guided, gameOver, finish]);

  if (!tip) return null;
  const copy = tipKeys(tip);
  return (
    <aside className={styles.coach} aria-live="polite" aria-label={t('match.coach.tip')}>
      <h2 className={styles.title}>{t(copy.title)}</h2>
      <p className={styles.text}>{t(copy.text)}</p>
      <div className={styles.actions}>
        <Button variant="quiet" size="sm" onClick={finish}>{t('match.lesson.quiet')}</Button>
        <Button variant="print" size="sm" onClick={() => markSeen(tip)}>{t('match.lesson.gotIt')}</Button>
      </div>
    </aside>
  );
}

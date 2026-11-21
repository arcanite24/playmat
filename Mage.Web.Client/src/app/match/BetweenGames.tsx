import { useState } from 'react';
import type { MatchProgress } from '../../core/game/matchProgress';
import { useT } from '../i18n';
import { Button } from '../ui/Button';
import './matchMessages';
import styles from './BetweenGames.module.css';

/**
 * The pause between games of a match: the result, the score, and the next game on its way. The server starts the
 * next game (or sideboarding) by itself, and the client follows; leaving here gives up the rest of the match.
 */
export function BetweenGames({ progress, won, onLeave, leaveLabel, concedes = true }: {
  progress: MatchProgress;
  /** null for a draw */
  won: boolean | null;
  onLeave(): void;
  leaveLabel?: string;
  /** leaving gives up the rest of the match (not in events, where the event decides) */
  concedes?: boolean;
}) {
  const t = useT();
  const [confirming, setConfirming] = useState(false);
  const result = t(won === null ? 'match.end.draw' : won ? 'match.between.won' : 'match.between.lost');
  const leave = leaveLabel ?? t('match.ui.leaveMatch');
  return (
    <div className={styles.scrim} role="dialog" aria-modal="true" aria-labelledby="between-games-title">
      <div className={styles.center}>
        <p className={styles.kicker}>{t('match.between.game', { number: progress.gameNumber, of: progress.bestOf })}</p>
        <h2 id="between-games-title" className={[styles.title, won ? styles.won : ''].join(' ')}>{result}</h2>
        <p className={styles.score} aria-label={t('match.between.score', { wins: progress.wins, losses: progress.losses })}>
          <span>{progress.wins}</span>
          <span className={styles.dash}>–</span>
          <span>{progress.losses}</span>
        </p>
        <p className={styles.next} role="status">
          {t('match.between.next')}<span className={styles.dots} aria-hidden="true" />
        </p>
        <p className={styles.hint}>{t('match.between.firstTo', { count: progress.winsNeeded })}</p>
        <div className={styles.leave}>
          {confirming ? (
            <>
              <span>{t('match.between.confirm')}</span>
              <Button variant="quiet" size="sm" onClick={() => setConfirming(false)}>{t('match.between.stay')}</Button>
              <Button variant="danger" size="sm" onClick={onLeave}>{leave}</Button>
            </>
          ) : (
            <Button variant="quiet" size="sm" onClick={() => (concedes ? setConfirming(true) : onLeave())}>{leave}</Button>
          )}
        </div>
      </div>
    </div>
  );
}

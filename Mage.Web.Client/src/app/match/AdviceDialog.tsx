import { useEffect, useState } from 'react';
import type { Advice } from '../../protocol/generated/views';
import { api } from '../connection';
import { formatNumber, useT, type MessageKey, type Translate } from '../i18n';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import './matchMessages';
import styles from './AdviceDialog.module.css';

const HEADINGS: Record<string, MessageKey> = {
  PLAY: 'match.advice.play',
  PASS: 'match.advice.pass',
  ATTACK: 'match.advice.attack',
  NO_ATTACK: 'match.advice.noAttack',
  BLOCK: 'match.advice.block',
  NO_BLOCK: 'match.advice.noBlock',
  NONE: 'match.advice.none',
};

/**
 * "What would the AI do?": the computer takes your seat in a copy of the game and says what it would do now, without
 * doing it. It thinks while the game waits for you, so ask when it's your decision.
 */
export function AdviceDialog({ gameId, open, onOpenChange, counted, onUsed }: {
  gameId: string;
  open: boolean;
  onOpenChange(open: boolean): void;
  /** a Career game: hints are counted and halve the pay, so each one is asked for on purpose */
  counted?: { hintsLeft: number } | null;
  /** a hint was given */
  onUsed?(): void;
}) {
  const t = useT();
  const [advice, setAdvice] = useState<Advice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [thinking, setThinking] = useState(false);

  async function ask() {
    setThinking(true);
    setError(null);
    setAdvice(null);
    try {
      setAdvice(await api.gameAdvise(gameId));
      onUsed?.();
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      setThinking(false);
    }
  }

  useEffect(() => {
    if (!open) return;
    setAdvice(null);
    setError(null);
    // a counted hint is only spent when the player asks for it
    if (!counted) void ask();
    // a new question each time it opens
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const steps = advice?.steps ?? [];
  const left = advice?.hintsLeft ?? counted?.hintsLeft;
  const canAsk = !counted || (left ?? 0) > 0;
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t(counted ? 'match.advice.hintTitle' : 'match.menu.askAi')}
      description={t(counted ? 'match.advice.hintText' : 'match.advice.text')}
      width="sm"
      footer={(
        <>
          <Button variant="quiet" onClick={() => onOpenChange(false)}>{t('ui.close')}</Button>
          {canAsk && (
            <Button variant="print" busy={thinking} onClick={() => void ask()}>
              {t(counted ? (advice ? 'match.advice.another' : 'match.advice.use') : 'match.advice.again')}
            </Button>
          )}
        </>
      )}
    >
      <div className={styles.body} aria-live="polite">
        {counted && left !== undefined && (
          <p className={styles.quiet}>{hintsLeftLabel(t, left)}</p>
        )}
        {thinking && <p className={styles.quiet}>{t('match.advice.thinking')}</p>}
        {error && <p className={styles.error} role="alert">{error}</p>}
        {advice && (
          <>
            <p className={styles.heading}>{t(HEADINGS[advice.kind ?? 'NONE'] ?? 'match.advice.title')}{advice.when ? <small> · {advice.when}</small> : null}</p>
            {steps.length > 1 ? (
              <ol className={styles.steps}>
                {steps.map((step, index) => <li key={`${step.objectId ?? ''}-${index}`}>{step.text}</li>)}
              </ol>
            ) : (
              <p className={styles.text}>{advice.text}</p>
            )}
            {advice.reason && <p className={styles.reason}>{t('match.advice.why', { reason: advice.reason })}</p>}
            {advice.millis !== undefined && <p className={styles.quiet}>{t('match.advice.thought', { seconds: formatNumber(advice.millis / 1000, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) })}</p>}
          </>
        )}
      </div>
    </Dialog>
  );
}

function hintsLeftLabel(t: Translate, left: number): string {
  return left <= 0 ? t('match.advice.noHints') : t('match.advice.hintsLeft', { count: left });
}

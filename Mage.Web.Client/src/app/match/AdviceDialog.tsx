import { useEffect, useState } from 'react';
import type { Advice } from '../../protocol/generated/views';
import { api } from '../connection';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import styles from './AdviceDialog.module.css';

const HEADINGS: Record<string, string> = {
  PLAY: 'The AI would play',
  PASS: 'The AI would pass',
  ATTACK: 'The AI would attack',
  NO_ATTACK: 'The AI wouldn\'t attack',
  BLOCK: 'The AI would block',
  NO_BLOCK: 'The AI wouldn\'t block',
  NONE: 'No advice',
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
      title={counted ? 'Ask for a hint' : 'Ask the AI'}
      description={counted
        ? 'The computer says what it would do in your seat. A Career game with a hint pays half and counts for no quests.'
        : 'What the computer would do in your seat right now. Nothing happens until you do it.'}
      width="sm"
      footer={(
        <>
          <Button variant="quiet" onClick={() => onOpenChange(false)}>Close</Button>
          {canAsk && (
            <Button variant="print" busy={thinking} onClick={() => void ask()}>
              {counted ? (advice ? 'Another hint' : 'Use a hint') : 'Ask again'}
            </Button>
          )}
        </>
      )}
    >
      <div className={styles.body} aria-live="polite">
        {counted && left !== undefined && (
          <p className={styles.quiet}>{hintsLeftLabel(left)}</p>
        )}
        {thinking && <p className={styles.quiet}>Thinking it over…</p>}
        {error && <p className={styles.error} role="alert">{error}</p>}
        {advice && (
          <>
            <p className={styles.heading}>{HEADINGS[advice.kind ?? 'NONE'] ?? 'Advice'}{advice.when ? <small> · {advice.when}</small> : null}</p>
            {steps.length > 1 ? (
              <ol className={styles.steps}>
                {steps.map((step, index) => <li key={`${step.objectId ?? ''}-${index}`}>{step.text}</li>)}
              </ol>
            ) : (
              <p className={styles.text}>{advice.text}</p>
            )}
            {advice.reason && <p className={styles.reason}>Why: {advice.reason}</p>}
            {advice.millis !== undefined && <p className={styles.quiet}>Thought for {(advice.millis / 1000).toFixed(1)} s.</p>}
          </>
        )}
      </div>
    </Dialog>
  );
}

function hintsLeftLabel(left: number): string {
  if (left <= 0) return 'No hints left in this game.';
  return left === 1 ? '1 hint left in this game.' : `${left} hints left in this game.`;
}

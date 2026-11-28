import { GitBranch } from 'lucide-react';
import { useState } from 'react';
import { useT } from '../i18n';
import '../match/matchMessages';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { playFromTurn, useSoloTurns } from './soloData';
import styles from './WhatIfDialog.module.css';

/**
 * What-if lines (S013, #423): pick a turn of this game to play again from its start, at a new table. The game you're
 * in is saved at the start of its current turn so you can resume it from Home's Saved games card.
 */
export function WhatIfDialog({ gameId, open, onOpenChange, currentTurn }: {
  gameId: string;
  open: boolean;
  onOpenChange(open: boolean): void;
  /** the live game's current turn number, shown for orientation (#423) */
  currentTurn?: number;
}) {
  const t = useT();
  const turns = useSoloTurns(gameId, open, currentTurn);
  const [busy, setBusy] = useState<number | null>(null);
  async function play(turn: number) {
    setBusy(turn);
    try {
      await playFromTurn(gameId, turn);
      notify(t('match.whatIf.title'), t('match.whatIf.started', { turn }));
      onOpenChange(false);
    } catch (failure) {
      notify(t('match.whatIf.title'), failure instanceof Error ? failure.message : String(failure), 'error');
    } finally {
      setBusy(null);
    }
  }
  const list = [...(turns.data ?? [])].reverse();
  const hasTurn = typeof currentTurn === 'number' && currentTurn > 0;
  const description = hasTurn
    ? t('match.whatIf.descriptionWithTurn', { turn: currentTurn })
    : t('match.whatIf.description');

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('match.whatIf.title')}
      description={description}
      width="sm"
      footer={<Button variant="quiet" onClick={() => onOpenChange(false)}>{t('match.whatIf.close')}</Button>}
    >
      {hasTurn && <p className={styles.current}>{t('match.whatIf.currentTurn', { turn: currentTurn })}</p>}
      {turns.isPending && <p className={styles.quiet} role="status">{t('match.whatIf.loading')}</p>}
      {turns.isError && <p className={styles.quiet} role="alert">{t('match.whatIf.failed')}</p>}
      {turns.data && list.length === 0 && <p className={styles.quiet}>{t('match.whatIf.empty')}</p>}
      {list.length > 0 && (
        <ul className={styles.turns}>
          {list.map((entry) => {
            const isCurrent = hasTurn && entry.turn === currentTurn;
            return (
              <li key={entry.turn} className={isCurrent ? styles.turnCurrent : undefined}>
                <span className={styles.turnLabel}>
                  <span>{t('match.turn', { turn: entry.turn ?? 0 })}{entry.active ? <small> · {entry.active}</small> : null}</span>
                  {isCurrent && <em className={styles.badge}>{t('match.whatIf.thisTurn')}</em>}
                </span>
                <Button size="sm" variant="print" icon={<GitBranch size={15} />} busy={busy === entry.turn} disabled={busy !== null} onClick={() => void play(entry.turn ?? 0)}>
                  {t('match.whatIf.playFromHere')}
                </Button>
              </li>
            );
          })}
        </ul>
      )}
    </Dialog>
  );
}

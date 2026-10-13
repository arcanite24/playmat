import { GitBranch } from 'lucide-react';
import { useState } from 'react';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { playFromTurn, useSoloTurns } from './soloData';
import styles from './WhatIfDialog.module.css';

/**
 * What-if lines (S013): pick a turn of this game to play again from its start, at a new table. The game you're in is
 * saved as it stands, so you can come back to it from Home.
 */
export function WhatIfDialog({ gameId, open, onOpenChange }: { gameId: string; open: boolean; onOpenChange(open: boolean): void }) {
  const turns = useSoloTurns(gameId, open);
  const [busy, setBusy] = useState<number | null>(null);
  async function play(turn: number) {
    setBusy(turn);
    try {
      await playFromTurn(gameId, turn);
      notify('What if', `A new game starts from the start of turn ${turn}. This one is saved on Home.`);
      onOpenChange(false);
    } catch (failure) {
      notify('What if', failure instanceof Error ? failure.message : String(failure));
    } finally {
      setBusy(null);
    }
  }
  const list = [...(turns.data ?? [])].reverse();
  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Play an earlier turn"
      description="Start a new game from the start of any turn so far, with the same cards in the same places. This game is saved as it stands, to resume from Home."
      width="sm"
      footer={<Button variant="quiet" onClick={() => onOpenChange(false)}>Close</Button>}
    >
      {turns.isPending && <p className={styles.quiet} role="status">Loading the turns…</p>}
      {turns.isError && <p className={styles.quiet} role="alert">The turns couldn't be loaded.</p>}
      {turns.data && list.length === 0 && <p className={styles.quiet}>No turn has been kept yet: the game keeps each turn as it starts.</p>}
      {list.length > 0 && (
        <ul className={styles.turns}>
          {list.map((entry) => (
            <li key={entry.turn}>
              <span>Turn {entry.turn}{entry.active ? <small> · {entry.active}</small> : null}</span>
              <Button size="sm" variant="print" icon={<GitBranch size={15} />} busy={busy === entry.turn} disabled={busy !== null} onClick={() => void play(entry.turn ?? 0)}>
                Play from here
              </Button>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}

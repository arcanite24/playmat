import { useQuery } from '@tanstack/react-query';
import { useId, useState } from 'react';
import { api } from '../connection';
import { notify } from '../stores/toasts';
import { SavePuzzle } from '../career/PuzzleExtras';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';
import styles from './PracticeTools.module.css';

type Tool = 'HAND' | 'BATTLEFIELD' | 'DRAW' | 'UNTAP_ALL' | 'LIFE';

const DONE: Record<Tool, string> = {
  HAND: 'Into your hand',
  BATTLEFIELD: 'Onto the battlefield',
  DRAW: 'Drawing',
  UNTAP_ALL: 'Untapping',
  LIFE: 'Setting your life',
};

/**
 * Practice tools for a game against the computer: any card into your hand or onto the battlefield, extra draws, an
 * untap and a life total. The server applies each the next time you are asked something and says so in the log.
 */
export function PracticeTools({ gameId, open, onOpenChange }: { gameId: string; open: boolean; onOpenChange(open: boolean): void }) {
  const [name, setName] = useState('');
  const [copies, setCopies] = useState(1);
  const [draws, setDraws] = useState(1);
  const [life, setLife] = useState(20);
  const [error, setError] = useState<string | null>(null);
  const listId = useId();
  const typed = name.trim();
  const suggestions = useQuery({
    queryKey: ['practice-names', typed.toLowerCase()],
    queryFn: () => api.searchCards({ nameContains: typed, uniqueNames: true, count: 12, start: 0 }),
    enabled: open && typed.length >= 3,
    staleTime: 60_000,
  });

  async function use(tool: Tool, amount: number, cardName?: string) {
    setError(null);
    try {
      await api.gamePractice(gameId, tool, cardName ?? null, amount);
      notify('Practice', `${DONE[tool]} when you're next asked to act.`);
      if (tool === 'HAND' || tool === 'BATTLEFIELD') setName('');
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Practice tools"
      description="Only in games against the computer. Each change is noted in the game log."
      width="sm"
    >
      <form
        className={styles.section}
        onSubmit={(event) => {
          event.preventDefault();
          if (typed) void use('HAND', copies, typed);
        }}
      >
        <Field label="Card" value={name} onChange={(event) => setName(event.target.value)} list={listId} autoComplete="off" placeholder="Lightning Bolt" />
        <datalist id={listId}>
          {(suggestions.data ?? []).map((card) => <option key={card.name} value={card.name} />)}
        </datalist>
        <div className={styles.row}>
          <Field label="Copies" type="number" min={1} max={10} value={copies} onChange={(event) => setCopies(Number(event.target.value) || 1)} className={styles.number} />
          <Button type="submit" variant="print" disabled={!typed}>To hand</Button>
          <Button type="button" variant="print" disabled={!typed} onClick={() => void use('BATTLEFIELD', copies, typed)}>To battlefield</Button>
        </div>
      </form>
      <div className={styles.row}>
        <Field label="Cards" type="number" min={1} max={10} value={draws} onChange={(event) => setDraws(Number(event.target.value) || 1)} className={styles.number} />
        <Button variant="print" onClick={() => void use('DRAW', draws)}>Draw</Button>
        <Button variant="print" onClick={() => void use('UNTAP_ALL', 0)}>Untap all</Button>
      </div>
      <div className={styles.row}>
        <Field label="Life" type="number" min={1} max={999} value={life} onChange={(event) => setLife(Number(event.target.value) || 1)} className={styles.number} />
        <Button variant="print" onClick={() => void use('LIFE', life)}>Set life</Button>
      </div>
      {error && <p className={styles.error} role="alert">{error}</p>}
      <SavePuzzle gameId={gameId} />
    </Dialog>
  );
}

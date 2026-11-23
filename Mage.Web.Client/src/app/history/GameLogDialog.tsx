import { useQuery } from '@tanstack/react-query';
import { Check, Copy, Search } from 'lucide-react';
import { useMemo, useState } from 'react';
import { buildLogRows, parseChatMessage, type LogEntry, type LogRow } from '../../core/game/gameLog';
import { loadTimeline } from '../review/reviewData';
import { useSession } from '../stores/session';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { PromptText } from '../ui/PromptText';
import styles from './GameLogDialog.module.css';

function formatPlainText(rows: readonly LogRow[]): string {
  const lines: string[] = [];
  for (const row of rows) {
    if (row.kind === 'separator') {
      lines.push(`--- Turn ${row.turn}${row.player ? ` · ${row.player}` : ''} ---`);
    } else if (row.entry.who) {
      lines.push(`${row.entry.who}: ${row.entry.text}`);
    } else {
      lines.push(row.entry.text);
    }
  }
  return lines.join('\n');
}

function filterRows(rows: readonly LogRow[], query: string): LogRow[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...rows];
  const result: LogRow[] = [];
  let pendingSeparator: LogRow | null = null;
  for (const row of rows) {
    if (row.kind === 'separator') {
      pendingSeparator = row;
      continue;
    }
    const hay = `${row.entry.who ?? ''} ${row.entry.text}`.toLowerCase();
    if (hay.includes(needle)) {
      if (pendingSeparator) {
        result.push(pendingSeparator);
        pendingSeparator = null;
      }
      result.push(row);
    }
  }
  return result;
}

/** Full turn-by-turn game log of a finished game, read from its recorded replay (#483). */
export function GameLogDialog({ gameId, title = 'Game log', onClose }: {
  gameId: string | null;
  title?: string;
  onClose(): void;
}) {
  const me = useSession((state) => state.userName);
  const [query, setQuery] = useState('');
  const [copied, setCopied] = useState(false);

  const timeline = useQuery({
    queryKey: ['timeline', gameId],
    queryFn: () => loadTimeline(gameId!),
    enabled: !!gameId,
    staleTime: Infinity,
    retry: 2,
    retryDelay: 1200,
  });

  const rows = useMemo(() => {
    const logs = timeline.data?.logs ?? [];
    const entries: LogEntry[] = [];
    for (let i = 0; i < logs.length; i++) {
      const parsed = parseChatMessage(logs[i], `h${i}`);
      if (parsed) entries.push(parsed);
    }
    return buildLogRows(entries, me);
  }, [timeline.data, me]);

  const shown = useMemo(() => filterRows(rows, query), [rows, query]);

  async function copyLog() {
    const text = formatPlainText(shown);
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      notify('Game log', 'Could not copy the log to the clipboard.', 'error');
    }
  }

  return (
    <Dialog
      open={gameId !== null}
      onOpenChange={(open) => {
        if (!open) {
          setQuery('');
          setCopied(false);
          onClose();
        }
      }}
      title={title}
      description="Complete turn-by-turn log recorded for this game."
      width="lg"
      footer={(
        <>
          <Button variant="print" size="sm" icon={copied ? <Check size={15} /> : <Copy size={15} />} disabled={shown.length === 0} onClick={() => void copyLog()}>
            {copied ? 'Copied' : 'Copy log'}
          </Button>
          <Button variant="quiet" size="sm" onClick={onClose}>Close</Button>
        </>
      )}
    >
      {timeline.isPending && <p className={styles.note} role="status">Loading the game log…</p>}
      {timeline.isError && <p className={styles.note} role="alert">The game log could not be loaded.</p>}
      {timeline.isSuccess && !timeline.data && <p className={styles.note}>No log was recorded for this game.</p>}
      {timeline.isSuccess && timeline.data && (
        <div className={styles.body}>
          <label className={styles.search}>
            <Search size={15} aria-hidden="true" />
            <input
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Filter by card, player, or action…"
              aria-label="Filter game log"
            />
          </label>
          {shown.length === 0 ? (
            <p className={styles.note}>{rows.length === 0 ? 'No log entries recorded.' : 'No log lines match that filter.'}</p>
          ) : (
            <ol className={styles.lines}>
              {shown.map((row) => (row.kind === 'separator' ? (
                <li key={row.key} className={[styles.separator, row.mine ? styles.separatorMine : ''].join(' ')}>
                  <span>Turn {row.turn}{row.player ? ` · ${row.mine ? 'You' : row.player}` : ''}</span>
                </li>
              ) : (
                <li key={row.key} className={styles.line}>
                  {row.entry.who && <strong className={styles.who}>{row.entry.who}: </strong>}
                  <PromptText text={row.entry.text} />
                </li>
              )))}
            </ol>
          )}
        </div>
      )}
    </Dialog>
  );
}

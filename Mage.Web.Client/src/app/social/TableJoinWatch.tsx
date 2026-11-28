import { useEffect, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { hostedFilling, newGuests, type TableGuest } from '../../core/social/tableJoins';
import type { TableView } from '../../protocol/generated/views';
import { queryClient, useTables } from '../queries';
import { attention } from '../stores/attention';
import { useSession } from '../stores/session';
import { useSettings } from '../stores/settings';
import { notify } from '../stores/toasts';

/**
 * Tells a host when someone sits down at their table (#209): a toast, a chime, and a browser notification while the
 * tab is in the background. It reads every refresh of the table list, wherever it comes from, and while one of the
 * player's tables is still filling it keeps the list refreshing on any screen, so the news doesn't wait for the
 * Tables screen.
 */
export function TableJoinWatch() {
  const me = useSession((state) => state.userName);
  const navigate = useNavigate();
  const onTables = useLocation().pathname.startsWith('/tables');
  const [filling, setFilling] = useState(false);
  // read by the subscription, which stays put while the player moves between screens
  const where = useRef({ onTables, navigate });
  useEffect(() => {
    where.current = { onTables, navigate };
  }, [onTables, navigate]);

  useEffect(() => {
    // the last list seen per room; a new player (another sign-in) starts over
    const seen = new Map<string, TableView[]>();
    return queryClient.getQueryCache().subscribe((event) => {
      if (event.type !== 'updated' || event.action.type !== 'success' || event.query.queryKey[0] !== 'tables') return;
      const tables = (event.query.state.data as TableView[] | undefined) ?? [];
      const key = event.query.queryHash;
      const guests = newGuests(seen.get(key), tables, me);
      seen.set(key, tables);
      setFilling(tables.some((table) => hostedFilling(table, me)));
      const { onTables: there, navigate: go } = where.current;
      for (const guest of guests) announce(guest, there ? null : () => go('/tables'));
    });
  }, [me]);

  return filling ? <KeepTablesFresh /> : null;
}

/** Keeps the table list polled while it is mounted (useTables refreshes a filling table's list in a background tab too). */
function KeepTablesFresh() {
  useTables();
  return null;
}

function announce(guest: TableGuest, openTables: (() => void) | null): void {
  const title = `${guest.name} joined your table`;
  const body = guest.open > 0
    ? `${guest.tableName}: ${guest.open} ${guest.open === 1 ? 'seat' : 'seats'} still open.`
    : `${guest.tableName} is full. Start when you're ready.`;
  notify(title, body, 'info', openTables ? { label: 'Open tables', run: openTables } : undefined);
  attention(title, body, { kind: 'tableJoins', tag: `table:${guest.tableId}`, onClick: openTables ?? undefined });
  if (useSettings.getState().settings.tableJoinSound) {
    // the game cues load with the first match; a host waiting at the lobby may not have them yet
    void import('../match/sound').then(({ playCue }) => playCue('turn')).catch(() => undefined);
  }
}

import type { TableView } from '../../protocol/generated/views';

/** A guest who sat down at a table the player hosts. */
export interface TableGuest {
  tableId: string;
  tableName: string;
  name: string;
  /** seats still empty after them */
  open: number;
}

/** The table's host: its controller name lists the host first ("Ann, Bob"). */
export function hostOf(table: TableView): string {
  return (table.controllerName ?? '').split(', ')[0];
}

/** A standalone table of this player's that still takes guests or waits for its Start. */
export function hostedFilling(table: TableView, me: string): boolean {
  return !!me && !table.isTournament && hostOf(table) === me
    && (table.tableState === 'WAITING' || table.tableState === 'READY_TO_START');
}

function people(table: TableView, me: string): string[] {
  return (table.seats ?? [])
    .filter((seat) => seat.playerName && seat.playerName !== me && (seat.playerType ?? 'HUMAN') === 'HUMAN')
    .map((seat) => seat.playerName!);
}

/**
 * The people who sat down at the player's own tables between two looks at the table list (#209). A table seen for the
 * first time announces nobody: whoever sits there was already there (a reload), and AI seats are the host's own doing.
 */
export function newGuests(before: readonly TableView[] | undefined, after: readonly TableView[], me: string): TableGuest[] {
  if (!before || !me) return [];
  const earlier = new Map(before.filter((table) => table.tableId).map((table) => [table.tableId!, table]));
  const guests: TableGuest[] = [];
  for (const table of after) {
    if (!table.tableId || !hostedFilling(table, me)) continue;
    const previous = earlier.get(table.tableId);
    if (!previous) continue;
    const seated = new Set(people(previous, me).map((name) => name.toLowerCase()));
    const open = (table.seats ?? []).filter((seat) => !seat.playerName).length;
    for (const name of people(table, me)) {
      if (!seated.has(name.toLowerCase())) guests.push({ tableId: table.tableId, tableName: table.tableName ?? 'your table', name, open });
    }
  }
  return guests;
}

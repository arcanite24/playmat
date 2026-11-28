import { describe, expect, test } from 'vitest';
import type { SeatView, TableView } from '../../protocol/generated/views';
import { hostedFilling, newGuests } from './tableJoins';

const seat = (playerName?: string, playerType: SeatView['playerType'] = 'HUMAN'): SeatView => ({ playerName, playerType });

function table(seats: SeatView[], patch: Partial<TableView> = {}): TableView {
  return { tableId: 't1', tableName: 'Friday', controllerName: 'ann', tableState: 'WAITING', isTournament: false, seats, ...patch };
}

describe('newGuests (#209)', () => {
  test('a person sitting down at my table is news, with the seats still open', () => {
    const before = [table([seat('ann'), seat(), seat()])];
    const after = [table([seat('ann'), seat('Bob'), seat()])];
    expect(newGuests(before, after, 'ann')).toEqual([{ tableId: 't1', tableName: 'Friday', name: 'Bob', open: 1 }]);
  });

  test('the first look, a table seen for the first time, and someone already seated announce nobody', () => {
    const now = [table([seat('ann'), seat('Bob')])];
    expect(newGuests(undefined, now, 'ann')).toEqual([]);
    expect(newGuests([], now, 'ann')).toEqual([]);
    expect(newGuests(now, now, 'ann')).toEqual([]);
  });

  test("only tables I host: not someone else's, not events, not AI seats, not once the game is on", () => {
    const empty = [seat('ann'), seat()];
    const joined = [seat('ann'), seat('Bob')];
    expect(newGuests([table(empty, { controllerName: 'cat' })], [table(joined, { controllerName: 'cat' })], 'ann')).toEqual([]);
    expect(newGuests([table(empty, { isTournament: true })], [table(joined, { isTournament: true })], 'ann')).toEqual([]);
    expect(newGuests([table(empty)], [table([seat('ann'), seat('Computer 1', 'COMPUTER_MAD')])], 'ann')).toEqual([]);
    expect(newGuests([table(empty)], [table(joined, { tableState: 'DUELING' })], 'ann')).toEqual([]);
    expect(newGuests([table(empty)], [table(joined, { tableState: 'READY_TO_START' })], 'ann')).toHaveLength(1);
  });

  test('the host is the first name the table lists', () => {
    expect(hostedFilling(table([], { controllerName: 'ann, Bob' }), 'ann')).toBe(true);
    expect(hostedFilling(table([], { controllerName: 'ann, Bob' }), 'Bob')).toBe(false);
    expect(hostedFilling(table([]), '')).toBe(false);
  });
});

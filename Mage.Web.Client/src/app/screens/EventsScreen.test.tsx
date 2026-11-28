import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import type { TableView } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useSession } from '../stores/session';
import { EventsScreen } from './EventsScreen';

function tournamentTable(overrides: Partial<TableView>): TableView {
  return {
    tableId: 't-1',
    tableName: "wolfy's sealed",
    gameType: 'Sealed Swiss 3 Rounds',
    additionalInfoShort: 'Wins: 1, Constr.: 20m',
    isTournament: true,
    limited: true,
    seatsInfo: '6/6',
    tableState: 'READY_TO_START',
    tableStateText: 'Waiting to start',
    controllerName: 'wolfy, p2, p3, p4, p5, p6',
    seats: [
      { playerName: 'wolfy' },
      { playerName: 'p2' },
      { playerName: 'p3' },
      { playerName: 'p4' },
      { playerName: 'p5' },
      { playerName: 'p6' },
    ],
    ...overrides,
  };
}

function renderEvents(tables: TableView[], userName: string) {
  useSession.setState({ userName, roomId: 'room-1' });
  queryClient.setQueryData(['tables', 'room-1'], tables);
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <EventsScreen />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe('EventsScreen', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    queryClient.clear();
  });

  test('lets the host start a full READY_TO_START event from Open to join instead of Under way', async () => {
    const startSpy = vi.spyOn(api, 'tournamentStart').mockResolvedValue(true);
    const ready = tournamentTable({ tableId: 't-ready', tableState: 'READY_TO_START' });
    renderEvents([ready], 'wolfy');

    expect(screen.getByText('Nothing running right now.')).toBeTruthy();
    expect(screen.getByText('Ready to start')).toBeTruthy();

    const startButton = screen.getByRole('button', { name: 'Start' }) as HTMLButtonElement;
    expect(startButton.disabled).toBe(false);
    expect(screen.getByRole('button', { name: 'Close' })).toBeTruthy();

    fireEvent.click(startButton);
    await waitFor(() => expect(startSpy).toHaveBeenCalledWith('room-1', 't-ready'));
  });

  test('disables Start for host while WAITING and shows Leave for seated guests', () => {
    const waiting = tournamentTable({
      tableId: 't-wait',
      tableState: 'WAITING',
      tableStateText: 'Waiting (2/6)',
      seatsInfo: '2/6',
      controllerName: 'wolfy, p2',
      seats: [{ playerName: 'wolfy' }, { playerName: 'p2' }, {}, {}, {}, {}],
    });

    const { unmount } = renderEvents([waiting], 'wolfy');
    expect((screen.getByRole('button', { name: 'Start' }) as HTMLButtonElement).disabled).toBe(true);
    unmount();

    renderEvents([waiting], 'p2');
    expect(screen.getByRole('button', { name: 'Leave' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Start' })).toBeNull();
  });
});

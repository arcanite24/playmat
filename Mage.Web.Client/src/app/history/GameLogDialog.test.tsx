// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import type { ChatMessage } from '../../protocol/generated/views';

const loadTimeline = vi.fn();
vi.mock('../review/reviewData', () => ({ loadTimeline: (...args: unknown[]) => loadTimeline(...args) }));
vi.mock('../connection', async () => (await import('../../test/fakeConnection')).createFakeConnection());

const { GameLogDialog } = await import('./GameLogDialog');

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

const msg = (message: string, turnInfo = 'T1.M1'): ChatMessage => ({
  message,
  turnInfo,
  messageType: 'GAME',
  time: 1000,
} as unknown as ChatMessage);

describe('GameLogDialog (#483)', () => {
  test('renders recorded turn logs, filters entries, and copies plain text', async () => {
    loadTimeline.mockResolvedValue({
      frames: [],
      read: 3,
      logs: [
        msg('TURN 1 for Alice', 'T1.U'),
        msg('Alice casts Lightning Bolt', 'T1.M1'),
        msg('TURN 2 for Bob', 'T2.U'),
        msg('Bob plays Forest', 'T2.M1'),
      ],
    });
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });

    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <GameLogDialog gameId="g1" onClose={vi.fn()} />
      </QueryClientProvider>,
    );

    expect(await screen.findByText(/Alice casts Lightning Bolt/)).toBeTruthy();
    expect(screen.getByText(/Bob plays Forest/)).toBeTruthy();

    fireEvent.change(screen.getByRole('searchbox', { name: 'Filter game log' }), { target: { value: 'Bolt' } });
    expect(screen.getByText(/Alice casts Lightning Bolt/)).toBeTruthy();
    expect(screen.queryByText(/Bob plays Forest/)).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Copy log' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('--- Turn 1 · Alice ---\nAlice casts Lightning Bolt'));
  });
});

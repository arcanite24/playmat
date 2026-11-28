// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { WhatIfDialog } from './WhatIfDialog';

const soloTurns = vi.fn();

vi.mock('../connection', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../connection')>();
  return {
    ...actual,
    api: {
      ...actual.api,
      soloTurns: (...args: unknown[]) => soloTurns(...args),
      soloWhatIf: vi.fn(),
    },
  };
});

afterEach(() => {
  cleanup();
  soloTurns.mockReset();
});

describe('WhatIfDialog (#423)', () => {
  test('shows the current turn, clear Saved games on Home copy, and lists saved turns newest first', async () => {
    soloTurns.mockResolvedValue([
      { turn: 5, active: 'Alice' },
      { turn: 6, active: 'AI Opponent' },
      { turn: 7, active: 'Alice' },
    ]);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <WhatIfDialog gameId="g1" open onOpenChange={vi.fn()} currentTurn={7} />
      </QueryClientProvider>,
    );

    expect(screen.getByText('Currently on turn 7')).toBeTruthy();
    expect(screen.getByText(/Saved games card on Home/i)).toBeTruthy();
    expect(await screen.findByText('Current turn')).toBeTruthy();

    const items = screen.getAllByRole('listitem');
    expect(items.map((item) => item.textContent)).toEqual([
      expect.stringContaining('Turn 7'),
      expect.stringContaining('Turn 6'),
      expect.stringContaining('Turn 5'),
    ]);
  });
});

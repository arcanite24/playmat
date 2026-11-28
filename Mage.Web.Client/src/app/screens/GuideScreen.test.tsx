// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { api } from '../connection';
import { useDecks } from '../stores/decks';
import { usePlay } from '../stores/play';
import { GuideScreen } from './GuideScreen';

function renderGuide(career: boolean) {
  vi.spyOn(api, 'serverInfo').mockResolvedValue({ career } as never);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(['serverInfo'], { career });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/guide']}>
        <GuideScreen />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe('GuideScreen (#468)', () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  test('shows core gameplay basics, switches to formats tab, and exposes Academy when Career is on', () => {
    renderGuide(true);

    expect(screen.getByRole('heading', { name: 'How to play & formats' })).toBeTruthy();
    expect(screen.getByText('Turn structure')).toBeTruthy();
    expect(screen.getByText('The stack & timing')).toBeTruthy();
    expect(screen.getByText('Returning after a long break?')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Open Academy' })).toBeTruthy();

    fireEvent.click(screen.getByRole('tab', { name: 'Formats explained' }));
    expect(screen.getByText('Standard')).toBeTruthy();
    expect(screen.getByText('Pioneer')).toBeTruthy();
    expect(screen.getByText('Modern')).toBeTruthy();
    expect(screen.getByText('Commander (EDH)')).toBeTruthy();
    expect(screen.getByText('Draft & Sealed (Limited)')).toBeTruthy();
  });

  test('hides Academy when Career is off and starts the guided first game on click', async () => {
    useDecks.setState({ loaded: true, selectedId: 'starter:ajani.dck', saved: [], starters: [], metaDecks: [] });
    const playVsAi = vi.fn().mockResolvedValue(undefined);
    usePlay.setState({ phase: 'idle', playVsAi });

    renderGuide(false);
    expect(screen.queryByRole('button', { name: 'Open Academy' })).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'Start guided game' }));
    expect(playVsAi).toHaveBeenCalledWith('starter:ajani.dck', { guided: true });
  });
});

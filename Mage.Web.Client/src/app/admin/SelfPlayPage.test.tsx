import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createApi } from '../../protocol/generated/api';
import type { SelfPlayView } from '../../protocol/generated/views';
import { shortImage } from './adminFormat';
import { SelfPlayPage } from './SelfPlayPage';

const HOUR = 3600;
const now = Math.floor(Date.now() / 1000);
const firstHour = (Math.floor(now / HOUR) - 23) * HOUR;

function view(over: Partial<SelfPlayView> = {}): SelfPlayView {
  return {
    installed: true,
    running: true,
    updatedAt: now - 30,
    state: 'playing',
    detail: 'job loop5-r1 c000003, seeds 1500450+',
    job: { id: 'loop5-r1', tag: 1, image: 'ghcr.io/arcanite24/playmat-selfplay:sha-80ed27e@sha256:5d89f7700be0' },
    effective: { paused: false, quietUtc: '', cpuMax: 25, gamesMax: 4, jobs: 3, chunkGames: 150 },
    games24h: 300,
    games7d: 900,
    chunksDone24h: 2,
    chunksStopped24h: 1,
    seconds24h: 240,
    spoolMb: 2,
    chunksWaiting: 1,
    totals: { games: 900, chunksDone: 6, chunksStopped: 1 },
    hours: Array.from({ length: 24 }, (_, i) => ({
      start: firstHour + i * HOUR,
      games: i === 23 ? 150 : i === 22 ? 150 : 0,
      chunksDone: i >= 22 ? 1 : 0,
      chunksStopped: i === 23 ? 1 : 0,
    })),
    recent: [
      { at: now - 100, job: 'loop5-r1', chunk: 'c000002', outcome: 'stopped', games: 0, seconds: 40, reason: 'the server uses 33% CPU' },
      { at: now - 400, job: 'loop5-r1', chunk: 'c000001', outcome: 'done', games: 150, seconds: 83 },
    ],
    trainer: {
      run: 'loop5', round: 1, rounds: 1, phase: 'selfplay', at: now - 60, localGames: 20000,
      history: [{ round: 0, accepted: true, elo: 'elo +5.2 [-2.6, +13.1]', remoteGames: 1800, selfplaySeconds: 2500 }],
    },
    ...over,
  };
}

function renderPage(call: Parameters<typeof createApi>[0]['call']) {
  const api = createApi({ call });
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <SelfPlayPage api={api} />
    </QueryClientProvider>,
  );
}

describe('the Self-play section', () => {
  it('shows the worker, the day by hour, the trainer and the last chunks', async () => {
    renderPage(vi.fn().mockResolvedValue(view()));
    expect(await screen.findByText('Playing')).toBeTruthy();
    expect(screen.getByText(/job loop5-r1 c000003/)).toBeTruthy();
    expect(screen.getByText('sha-80ed27e')).toBeTruthy();
    const chart = screen.getByRole('list', { name: /Games played each hour \(UTC\), 300 in all/ });
    const bars = within(chart).getAllByRole('listitem');
    expect(bars).toHaveLength(24);
    expect(bars[23].getAttribute('aria-label')).toMatch(/150 games, 1 chunks thrown away/);
    expect(screen.getByText(/round 1 of 1: selfplay/)).toBeTruthy();
    expect(screen.getByText('elo +5.2 [-2.6, +13.1]')).toBeTruthy();
    expect(screen.getByText('thrown away')).toBeTruthy();
    expect(screen.getByText(/the server uses 33% CPU/)).toBeTruthy();
  });

  it('says when the worker stopped reporting', async () => {
    renderPage(vi.fn().mockResolvedValue(view({ running: false, updatedAt: now - 3 * HOUR })));
    expect(await screen.findByText('Not running')).toBeTruthy();
    expect(screen.getByText(/timer isn't running/)).toBeTruthy();
  });

  it('says how to set it up when the volume is missing', async () => {
    renderPage(vi.fn().mockResolvedValue({ installed: false }));
    expect(await screen.findByText(/isn't set up on this server/)).toBeTruthy();
  });

  it('pauses with the settings in the form', async () => {
    const call = vi.fn().mockImplementation((method: string) =>
      Promise.resolve(method === 'adminSelfPlaySet' ? view({ console: { paused: true, cpuMax: 40, by: 'neri', at: now } }) : view()));
    renderPage(call);
    fireEvent.change(await screen.findByLabelText(/Server CPU limit/), { target: { value: '40' } });
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    await waitFor(() => expect(call).toHaveBeenCalledWith('adminSelfPlaySet', true, '', 40, 4, 3));
    expect(await screen.findByRole('button', { name: 'Resume' })).toBeTruthy();
  });

  it('shortens image names', () => {
    expect(shortImage('ghcr.io/arcanite24/playmat-selfplay:sha-80ed27e@sha256:5d89f7700be024f4')).toBe('sha-80ed27e');
    expect(shortImage('ghcr.io/arcanite24/playmat-selfplay@sha256:5d89f7700be024f4885d')).toBe('sha256:5d89f7700be0');
  });
});

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SwarmScreen } from './SwarmScreen';

function renderAt(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/swarm" element={<SwarmScreen />} />
          <Route path="/swarm/contributors/:name" element={<SwarmScreen />} />
          <Route path="/swarm/:section" element={<SwarmScreen />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function mockFetch(routes: Record<string, unknown>) {
  return vi.spyOn(globalThis, 'fetch').mockImplementation(async (input) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
    for (const [suffix, body] of Object.entries(routes)) {
      if (url.endsWith(suffix)) {
        return new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } });
      }
    }
    return new Response(JSON.stringify({ error: `unmocked ${url}` }), { status: 404 });
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('SwarmScreen public tracker', () => {
  it('renders live overview KPIs, active round progress, and accepted feed', async () => {
    mockFetch({
      '/api/v1/status': {
        hosts: 3,
        threads: 28,
        units: { accepted: 19, queued: 5 },
        games_accepted: 1900,
        games_last_hour: 420,
        total_decisions: 69181,
        open_inbox: 0,
        platforms: { 'linux-x86_64': 2, 'macos-aarch64': 1 },
        active_round: {
          id: 3,
          project: 'mana',
          run: 's1-gate',
          round: 1,
          priority: 10,
          trained: false,
          created: 1795600000,
          units: { accepted: 19, queued: 5 },
          games: 1900,
          verify_pending: 1,
        },
      },
      '/api/v1/feed?limit=25': [
        {
          unit: 'mana-r1-u000',
          project: 'mana',
          run: 's1-gate',
          round: 1,
          kind: 'selfplay',
          account: 'brandon',
          host: 'minivac',
          games: 100,
          credit: 145.5,
          done_at: Math.floor(Date.now() / 1000) - 90,
        },
      ],
    });

    renderAt('/swarm');
    expect(await screen.findByText('1,900')).toBeTruthy();
    expect(screen.getByText('69,181')).toBeTruthy();
    expect(screen.getByText(/Active round: mana\/s1-gate round 1/)).toBeTruthy();
    expect(screen.getByText('linux-x86_64: 2')).toBeTruthy();
    expect(await screen.findByText('mana-r1-u000')).toBeTruthy();
    expect(screen.getByText('brandon')).toBeTruthy();
  });

  it('switches between contributors and teams on the leaderboard', async () => {
    mockFetch({
      '/api/v1/leaderboard?window=all': [
        {
          account: 'alice',
          trust: 'reputable',
          team: 'Azorius Compute',
          accepted_units: 42,
          games: 4200,
          verified_units: 12,
          credit: 520.4,
          credit_pending: 20,
          credit_matured: 500.4,
          credit_frozen: 0,
        },
      ],
      '/api/v1/teams?window=all': [
        {
          id: 1,
          name: 'Azorius Compute',
          description: 'Control players donating CPU',
          created: 1795600000,
          members: 4,
          accepted_units: 120,
          games: 12000,
          credit: 1450.2,
        },
      ],
    });

    renderAt('/swarm/leaderboard');
    expect(await screen.findByText('alice')).toBeTruthy();
    expect(screen.getByText('reputable')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Teams' }));
    expect(await screen.findByText('Azorius Compute')).toBeTruthy();
    expect(screen.getByText('Control players donating CPU')).toBeTruthy();
  });

  it('renders a contributor profile with badges and hosts', async () => {
    mockFetch({
      '/api/v1/contributors/alice': {
        account: 'alice',
        trust: 'trusted',
        team: 'Azorius Compute',
        created: 1795500000,
        accepted_units: 25,
        games: 2500,
        verified_units: 15,
        credit: 310.0,
        credit_pending: 10.0,
        credit_matured: 300.0,
        credit_frozen: 0.0,
        badges: ['First Spark', 'Probation Cleared', '1K Games', 'Verifier', 'Trusted'],
        hosts: [
          { name: 'alice-mac', os: 'macos-aarch64', class: 'strict-1', threads: 8, seen: 1795680000, accepted_units: 25 },
        ],
        recent: [],
      },
    });

    renderAt('/swarm/contributors/alice');
    expect(await screen.findByText('alice-mac')).toBeTruthy();
    expect(screen.getByText('Probation Cleared')).toBeTruthy();
    expect(screen.getByText('Team: Azorius Compute')).toBeTruthy();
  });

  it('renders networks lineage, Archbench Elo chart, and SPRT gates', async () => {
    mockFetch({
      '/api/v1/networks': [
        {
          sha256: '2'.repeat(64),
          label: 'gen-2',
          parent_sha: '1'.repeat(64),
          status: 'promoted',
          created: 1795680000,
          promoted_at: 1795681000,
          download_url: 'https://mana.deimos.studio/api/v1/nets/' + '2'.repeat(64),
          gate_id: 4,
          gate_status: 'promoted',
          gate_elo: 42.5,
          gate_llr: 3.12,
          bench_elo: 1125.0,
          bench_ci95: 38.0,
          bench_games: 200,
          anchors: [{ opponent: 'host@s6', wins: 132, draws: 4, losses: 64, elo: 1125.0, ci95: 38.0 }],
        },
        {
          sha256: '1'.repeat(64),
          label: 'gen-1',
          parent_sha: null,
          status: 'superseded',
          created: 1795600000,
          promoted_at: 1795600000,
          download_url: 'https://mana.deimos.studio/api/v1/nets/' + '1'.repeat(64),
          gate_id: null,
          gate_status: null,
          gate_elo: null,
          gate_llr: null,
          bench_elo: 1030.0,
          bench_ci95: 40.0,
          bench_games: 200,
          anchors: [],
        },
      ],
      '/api/v1/gates': [
        {
          id: 4,
          round_id: 10,
          project: 'main',
          run: 's4',
          round: 2,
          candidate_sha: '2'.repeat(64),
          candidate_label: 'gen-2',
          baseline_sha: '1'.repeat(64),
          baseline_label: 'gen-1',
          status: 'promoted',
          held: false,
          auto_promote: true,
          elo0: 0,
          elo1: 10,
          alpha: 0.05,
          beta: 0.05,
          lower_bound: -2.944,
          upper_bound: 2.944,
          wins: 140,
          draws: 10,
          losses: 90,
          elo: 42.5,
          elo_ci95: 35.0,
          llr: 3.12,
          max_share: 0.5,
          top_account_share: 0.35,
          created: 1795680000,
          decided_at: 1795681000,
          decision_reason: 'SPRT passed',
        },
      ],
    });

    renderAt('/swarm/networks');
    expect(await screen.findByText(/Gate #4: gen-2 vs gen-1/)).toBeTruthy();
    expect(screen.getByRole('img', { name: /Archbench Elo over network generations/ })).toBeTruthy();
    expect(screen.getByText(/1125 \+\/- 38 \(200g\)/)).toBeTruthy();
    expect(screen.getByText(/host@s6: 132-4-64/)).toBeTruthy();
  });
});

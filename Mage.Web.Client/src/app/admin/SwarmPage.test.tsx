import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SwarmPage } from './SwarmPage';

function renderSwarmAdmin() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <SwarmPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

afterEach(() => {
  sessionStorage.clear();
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('SwarmPage admin console', () => {
  it('connects with an admin token, loads inbox, and resolves an item', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
      if (url.endsWith('/api/v1/status')) {
        return new Response(
          JSON.stringify({
            hosts: 2,
            threads: 16,
            units: { accepted: 10, queued: 1, quarantined: 2 },
            games_accepted: 1000,
            games_last_hour: 200,
            total_decisions: 32000,
            open_inbox: 1,
            platforms: {},
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      if (url.endsWith('/api/v1/admin/inbox?status=open')) {
        return new Response(
          JSON.stringify([
            {
              id: 7,
              created: 1795680000,
              status: 'open',
              kind: 'rerun-mismatch',
              token: 5,
              account: 'cheater',
              host: 9,
              host_name: 'bad-box',
              unit: 'r2-u001',
              verify_unit: 'v-rerun-r2-u001',
              detail: 'steps_sha256 mismatch',
              resolved_by: null,
              resolved_at: null,
              resolution: null,
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      if (url.endsWith('/api/v1/admin/inbox/7/resolve') && init?.method === 'POST') {
        return new Response(null, { status: 204 });
      }
      return new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } });
    });

    renderSwarmAdmin();
    fireEvent.change(screen.getByLabelText('Admin bearer token'), { target: { value: 'mana_admin_secret' } });
    fireEvent.click(screen.getByRole('button', { name: 'Connect coordinator' }));

    expect(await screen.findByText('rerun-mismatch')).toBeTruthy();
    expect(screen.getByText(/steps_sha256 mismatch/)).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Confirm cheat' }));
    expect(await screen.findByText(/Inbox item #7 resolved \(confirm-cheat\)/)).toBeTruthy();
    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/admin/inbox/7/resolve'),
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('holds a gate and rolls back a network with a written reason', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
      const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url;
      if (url.endsWith('/api/v1/status')) {
        return new Response(
          JSON.stringify({
            hosts: 1,
            threads: 8,
            units: { accepted: 5 },
            games_accepted: 200,
            games_last_hour: 50,
            total_decisions: 1000,
            open_inbox: 0,
            platforms: {},
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      if (url.endsWith('/api/v1/gates')) {
        return new Response(
          JSON.stringify([
            {
              id: 3,
              round_id: 9,
              project: 'main',
              run: 's4',
              round: 1,
              candidate_sha: 'b'.repeat(64),
              candidate_label: 'gen-b',
              baseline_sha: 'a'.repeat(64),
              baseline_label: 'gen-a',
              status: 'running',
              held: false,
              auto_promote: true,
              elo0: 0,
              elo1: 10,
              alpha: 0.05,
              beta: 0.05,
              lower_bound: -2.944,
              upper_bound: 2.944,
              wins: 20,
              draws: 0,
              losses: 15,
              elo: 25,
              elo_ci95: 80,
              llr: 0.5,
              max_share: 0.5,
              top_account_share: 0.4,
              created: 1795680000,
              decided_at: null,
              decision_reason: '',
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      if (url.endsWith('/api/v1/networks')) {
        return new Response(
          JSON.stringify([
            {
              sha256: 'a'.repeat(64),
              label: 'gen-a',
              parent_sha: null,
              status: 'superseded',
              created: 1795600000,
              promoted_at: 1795600000,
              download_url: 'https://mana.deimos.studio/api/v1/nets/' + 'a'.repeat(64),
              gate_id: null,
              gate_status: null,
              gate_elo: null,
              gate_llr: null,
              bench_elo: 1015,
              bench_ci95: 40,
              bench_games: 100,
              anchors: [],
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      if (url.endsWith('/api/v1/admin/gates/3/hold') && init?.method === 'POST') {
        return new Response(
          JSON.stringify({ id: 3, status: 'held', candidate_label: 'gen-b' }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      }
      if (url.endsWith('/api/v1/admin/networks/rollback') && init?.method === 'POST') {
        return new Response(null, { status: 204 });
      }
      return new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } });
    });

    sessionStorage.setItem('mana.admin.token', 'mana_admin_secret');
    renderSwarmAdmin();

    fireEvent.click(screen.getByRole('tab', { name: 'Gates & Networks' }));
    expect(await screen.findByText('gen-b')).toBeTruthy();

    fireEvent.change(screen.getByLabelText('Action reason'), { target: { value: 'checking OOD deck' } });
    fireEvent.click(screen.getByRole('button', { name: 'Hold' }));
    expect(await screen.findByText(/Gate #3 held/)).toBeTruthy();

    fireEvent.change(screen.getByLabelText('Action reason'), { target: { value: 'rollback OOD' } });
    fireEvent.click(screen.getByRole('button', { name: 'Rollback here' }));
    expect(await screen.findByText(/Rolled back active network/)).toBeTruthy();
    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/admin/networks/rollback'),
      expect.objectContaining({ method: 'POST' }),
    );
  });
});

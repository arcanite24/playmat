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
});

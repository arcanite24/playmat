// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { createFakeConnection } from '../test/fakeConnection';

async function load() {
  vi.resetModules();
  const connection = createFakeConnection();
  vi.doMock('./connection', () => connection);
  const { MaintenanceChip } = await import('./AppShell');
  const { useServerInfo } = await import('./queries');
  const { useSession } = await import('./stores/session');
  return { MaintenanceChip, useServerInfo, useSession, connection };
}

describe('MaintenanceChip and useServerInfo', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  test('MaintenanceChip switches to maintenance-now as time passes without a refetch', async () => {
    vi.useFakeTimers();
    const start = new Date('2026-11-22T20:00:00Z').getTime();
    vi.setSystemTime(start);

    const { MaintenanceChip, useSession, connection } = await load();
    useSession.setState({ phase: 'signedIn', userName: 'alice', connection: 'open' });
    connection.api.serverInfo = vi.fn(async () => ({
      maintenanceAt: start + 25_000,
      maintenanceMessage: 'Quick restart',
    }));

    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <MaintenanceChip />
      </QueryClientProvider>,
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(connection.api.serverInfo).toHaveBeenCalledTimes(1);
    const chip = screen.getByRole('status');
    expect(chip.textContent).toContain('Maintenance at');

    // Advance 30s past maintenanceAt (still before the 60s refetchInterval): the clock tick flips the chip on its own
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(connection.api.serverInfo).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('status').textContent).toBe('Maintenance under way');
  });

  test('useServerInfo polls every minute while signed in', async () => {
    vi.useFakeTimers();
    const { useServerInfo, useSession, connection } = await load();
    useSession.setState({ phase: 'signedIn', userName: 'alice', connection: 'open' });
    connection.api.serverInfo = vi.fn(async () => ({ accounts: true }));

    function Probe() {
      useServerInfo();
      return null;
    }

    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <Probe />
      </QueryClientProvider>,
    );

    await act(async () => {
      await Promise.resolve();
    });
    expect(connection.api.serverInfo).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(connection.api.serverInfo).toHaveBeenCalledTimes(2);
  });
});

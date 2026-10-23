import { QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useSession } from '../stores/session';
import { useLooks, usePlayerLooks } from './supportData';

const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;

beforeEach(() => {
  vi.useFakeTimers();
  queryClient.clear();
  useSession.setState({ phase: 'signedIn', userName: 'Alice', loginCount: 1 });
  useLooks.setState({ looks: {} });
  vi.spyOn(api, 'profileLooks').mockResolvedValue([]);
  vi.spyOn(api, 'serverInfo').mockResolvedValue({ accounts: true });
});

afterEach(() => {
  queryClient.clear();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

test('looks wait for server capabilities, map both names case-insensitively, and refresh a late opponent upload', async () => {
  let resolveInfo!: (info: { accounts: boolean }) => void;
  vi.mocked(api.serverInfo).mockImplementation(() => new Promise((resolve) => { resolveInfo = resolve; }));
  vi.mocked(api.profileLooks).mockResolvedValue([{ name: 'ALICE', avatar: 'owl', frame: 'frame-bronze' }]);
  const { result, unmount } = renderHook(() => usePlayerLooks(['Alice', 'Bob'], true), { wrapper });
  await act(async () => { await vi.advanceTimersByTimeAsync(100); });
  expect(api.profileLooks).not.toHaveBeenCalled();
  await act(async () => {
    resolveInfo({ accounts: true });
    await vi.advanceTimersByTimeAsync(100);
  });
  expect(api.profileLooks).toHaveBeenCalledWith(['alice', 'bob']);
  expect(result.current.alice).toMatchObject({ avatar: 'owl', frame: 'frame-bronze' });
  expect(result.current.bob).toBeNull();
  vi.mocked(api.profileLooks).mockResolvedValue([
    { name: 'Alice', avatar: 'owl', frame: 'frame-bronze' },
    { name: 'BOB', avatar: 'fox', frame: 'frame-silver', sleeve: 'copper', cloth: 'copper' },
  ]);
  await act(async () => { await vi.advanceTimersByTimeAsync(30_100); });
  expect(result.current.bob).toMatchObject({ avatar: 'fox', frame: 'frame-silver', sleeve: 'copper', cloth: 'copper' });
  unmount();
  const calls = vi.mocked(api.profileLooks).mock.calls.length;
  await act(async () => { await vi.advanceTimersByTimeAsync(30_100); });
  expect(api.profileLooks).toHaveBeenCalledTimes(calls);
});

test('account servers fetch looks even with Career disabled and refresh after reconnecting', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: true, career: false });
  const { unmount } = renderHook(() => usePlayerLooks([' Alice ', 'BOB'], true), { wrapper });
  await act(async () => { await vi.advanceTimersByTimeAsync(100); });
  expect(api.profileLooks).toHaveBeenCalledWith(['alice', 'bob']);
  vi.mocked(api.profileLooks).mockClear();
  act(() => useSession.setState({ loginCount: 2 }));
  await act(async () => { await vi.advanceTimersByTimeAsync(100); });
  expect(api.profileLooks).toHaveBeenCalledWith(['alice', 'bob']);
  unmount();
});

test('servers without accounts do not request profile looks', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: false });
  const { unmount } = renderHook(() => usePlayerLooks(['Alice', 'Bob'], true), { wrapper });
  await act(async () => { await vi.advanceTimersByTimeAsync(30_100); });
  expect(api.profileLooks).not.toHaveBeenCalled();
  unmount();
});

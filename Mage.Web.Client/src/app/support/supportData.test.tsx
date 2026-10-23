import { QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { PlayerLook } from '../../protocol/generated/views';
import { api } from '../connection';
import { queryClient } from '../queries';
import { useSession } from '../stores/session';
import { rememberLook, useLook, useLooks, usePlayerLooks, wantLooks } from './supportData';

const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;

beforeEach(() => {
  vi.useFakeTimers();
  queryClient.clear();
  useSession.setState({ userName: '' });
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

test.each([{ found: [] }, { found: [{ name: 'ALICE', sleeve: 'copper', cloth: 'sand' }] }])('a late poll cannot replace a newer uploaded look with $found', async ({ found }) => {
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  let resolve!: (looks: PlayerLook[]) => void;
  vi.mocked(api.profileLooks).mockImplementationOnce(() => new Promise((done) => { resolve = done; }));
  wantLooks(['Alice', 'Bob'], true);
  await vi.advanceTimersByTimeAsync(50);
  const uploaded = { name: 'Alice', sleeve: '#20302b', cloth: 'navy' };
  rememberLook(uploaded);
  resolve([...found, { name: 'Bob', sleeve: 'copper' }]);
  await vi.advanceTimersByTimeAsync(0);
  expect(useLooks.getState().looks.alice).toBe(uploaded);
  expect(useLooks.getState().looks.bob?.sleeve).toBe('copper');
});

test('overlapping batches cannot land out of order for a name', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  let resolveFirst!: (looks: PlayerLook[]) => void;
  let resolveSecond!: (looks: PlayerLook[]) => void;
  vi.mocked(api.profileLooks)
    .mockImplementationOnce(() => new Promise((done) => { resolveFirst = done; }))
    .mockImplementationOnce(() => new Promise((done) => { resolveSecond = done; }));
  wantLooks(['Alice', 'Bob'], true);
  await vi.advanceTimersByTimeAsync(50);
  wantLooks(['BOB'], true);
  await vi.advanceTimersByTimeAsync(50);
  resolveSecond([{ name: 'Bob', sleeve: 'copper' }]);
  await vi.advanceTimersByTimeAsync(0);
  resolveFirst([{ name: 'Alice', cloth: 'navy' }, { name: 'Bob', sleeve: '#20302b' }]);
  await vi.advanceTimersByTimeAsync(0);
  expect(useLooks.getState().looks.alice?.cloth).toBe('navy');
  expect(useLooks.getState().looks.bob?.sleeve).toBe('copper');
});

test('an older failed request does not expire a newer remembered look', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  let reject!: (error: Error) => void;
  vi.mocked(api.profileLooks).mockImplementationOnce(() => new Promise((_, fail) => { reject = fail; }));
  wantLooks(['Alice'], true);
  await vi.advanceTimersByTimeAsync(50);
  rememberLook({ name: 'Alice', cloth: 'navy' });
  reject(new Error('offline'));
  await vi.advanceTimersByTimeAsync(0);
  wantLooks(['Alice']);
  await vi.advanceTimersByTimeAsync(50);
  expect(api.profileLooks).toHaveBeenCalledOnce();
  expect(useLooks.getState().looks.alice?.cloth).toBe('navy');
});

test('changing accounts cancels queued names and invalidates in-flight replies', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  let resolve!: (looks: PlayerLook[]) => void;
  vi.mocked(api.profileLooks).mockImplementationOnce(() => new Promise((done) => { resolve = done; }));
  wantLooks(['Alice'], true);
  await vi.advanceTimersByTimeAsync(50);
  wantLooks(['Bob'], true);
  useSession.setState({ userName: 'Carol' });
  resolve([{ name: 'Alice', cloth: 'navy' }]);
  await vi.advanceTimersByTimeAsync(50);
  expect(api.profileLooks).toHaveBeenCalledOnce();
  expect(useLooks.getState().looks).toEqual({});
});

test('unchanged polls keep the map and each look stable, including absent looks', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  vi.mocked(api.profileLooks).mockImplementation(async () => [{ name: 'Alice', sleeve: 'copper', cardStyles: { Forest: 'painted-forest' } }]);
  const render = vi.fn();
  const { result, unmount } = renderHook(() => {
    render();
    return usePlayerLooks(['Alice', 'Bob'], true);
  }, { wrapper });
  await act(async () => { await vi.advanceTimersByTimeAsync(100); });
  const previous = result.current;
  const renders = render.mock.calls.length;
  await act(async () => { await vi.advanceTimersByTimeAsync(30_100); });
  expect(api.profileLooks).toHaveBeenCalledTimes(2);
  expect(result.current).toBe(previous);
  expect(render).toHaveBeenCalledTimes(renders);
  expect(result.current.bob).toBeNull();
  vi.mocked(api.profileLooks).mockResolvedValue([
    { name: 'Alice', sleeve: 'copper', cardStyles: { Forest: 'painted-forest' } },
    { name: 'Bob', cloth: 'navy' },
  ]);
  await act(async () => { await vi.advanceTimersByTimeAsync(30_100); });
  expect(result.current).not.toBe(previous);
  expect(result.current.alice).toBe(previous.alice);
  expect(result.current.bob?.cloth).toBe('navy');
  unmount();
});

test('one player lookup does not subscribe to other players', async () => {
  queryClient.setQueryData(['serverInfo'], { accounts: true });
  const alice = { name: 'Alice', sleeve: 'copper' };
  rememberLook(alice);
  const render = vi.fn();
  const { result, unmount } = renderHook(() => {
    render();
    return useLook(' ALICE ');
  }, { wrapper });
  await act(async () => { await vi.advanceTimersByTimeAsync(100); });
  const renders = render.mock.calls.length;
  act(() => rememberLook({ name: 'Bob', cloth: 'navy' }));
  expect(render).toHaveBeenCalledTimes(renders);
  expect(result.current).toBe(alice);
  act(() => rememberLook({ name: 'Alice', sleeve: '#20302b' }));
  expect(result.current?.sleeve).toBe('#20302b');
  unmount();
});

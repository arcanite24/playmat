// @vitest-environment happy-dom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import type { GameSession } from '../../core/game/gameSession';
import { DEFAULT_SETTINGS, useSettings } from '../stores/settings';
import { useManaPaymentSettings } from './useManaPaymentSettings';

vi.mock('../connection', async () => (await import('../../test/fakeConnection')).createFakeConnection());

afterEach(() => {
  cleanup();
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS }, fullControl: false });
});

test('updates a running pool when floating mana is enabled, disabled or paused', async () => {
  useSettings.setState({ settings: { ...DEFAULT_SETTINGS, autoPayMana: false } });
  const respond = vi.fn<(command: unknown) => Promise<void>>(async () => {});
  const session = { respond } as unknown as GameSession;
  renderHook(() => useManaPaymentSettings(session, true));
  await act(async () => {});
  expect(respond.mock.calls.map(([command]) => command)).toEqual([
    { type: 'action', action: 'MANA_AUTO_PAYMENT_RESTRICTED_ON' },
    { type: 'action', action: 'MANA_AUTO_PAYMENT_OFF' },
  ]);

  act(() => useSettings.getState().update({ autoPayRestricted: false }));
  await act(async () => {});
  expect(respond).toHaveBeenLastCalledWith({ type: 'action', action: 'MANA_AUTO_PAYMENT_ON' });
  expect(respond).toHaveBeenNthCalledWith(3, { type: 'action', action: 'MANA_AUTO_PAYMENT_RESTRICTED_OFF' });

  act(() => useSettings.setState({ fullControl: true }));
  await act(async () => {});
  expect(respond).toHaveBeenLastCalledWith({ type: 'action', action: 'MANA_AUTO_PAYMENT_OFF' });
  act(() => useSettings.setState({ fullControl: false }));
  await act(async () => {});
  expect(respond).toHaveBeenLastCalledWith({ type: 'action', action: 'MANA_AUTO_PAYMENT_ON' });

  act(() => useSettings.getState().update({ autoPayRestricted: true }));
  await act(async () => {});
  expect(respond).toHaveBeenLastCalledWith({ type: 'action', action: 'MANA_AUTO_PAYMENT_OFF' });
});

test('does not send player actions before joining or while watching a game', async () => {
  const respond = vi.fn(async () => {});
  renderHook(() => useManaPaymentSettings({ respond } as unknown as GameSession, false));
  await act(async () => {});
  expect(respond).not.toHaveBeenCalled();
});

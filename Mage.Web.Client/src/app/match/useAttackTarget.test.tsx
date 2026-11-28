import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useStore } from 'zustand';
import { GameSession } from '../../core/game/gameSession';
import type { EventBus } from '../../core/rpc/EventBus';
import type { GameView } from '../../protocol/generated/views';
import { useAttackTarget } from './useAttackTarget';

function setup() {
  const handlers = new Map<string, (data: unknown, event: { objectId: string }) => void>();
  const bus = { on(method: string, handler: (data: unknown, event: { objectId: string }) => void) {
    handlers.set(method, handler);
    return () => handlers.delete(method);
  } } as unknown as EventBus;
  const api = { sendPlayerUUID: vi.fn(async () => true) };
  const session = new GameSession(api as never, bus, { gameId: 'game', playerId: 'me', mode: 'play' });
  const view = (attacking = false): GameView => ({
    myPlayerId: 'me', step: 'DECLARE_ATTACKERS',
    players: [
      { playerId: 'me', battlefield: { bear: { id: 'bear', cardTypes: ['CREATURE'], controllerId: 'me', tapped: attacking } } },
      { playerId: 'them', battlefield: {
        jace: { id: 'jace', cardTypes: ['PLANESWALKER'] },
        siege: { id: 'siege', cardTypes: ['BATTLE'] },
      } },
    ],
    combat: attacking ? [{ defenderId: 'them', attackers: { bear: { id: 'bear', controllerId: 'me' } } }] : [],
  });
  const emit = (method: string, data: unknown) => act(() => handlers.get(method)?.(data, { objectId: 'game' }));
  const declare = (attacking = false) => emit('GAME_SELECT', {
    message: 'Select attackers', gameView: view(attacking), options: { possibleAttackers: attacking ? [] : ['bear'] },
  });
  const target = (targets = ['them', 'jace', 'siege']) => emit('GAME_TARGET', {
    message: 'Select player, planeswalker, or battle to attack', gameView: view(), targets, flag: true,
  });
  declare();
  const hook = renderHook(() => useAttackTarget(session, useStore(session.store), 'me', true));
  return { session, api, hook, declare, target, emit, view };
}

afterEach(() => vi.useRealTimers());

describe('attacker defender flow', () => {
  it.each(['jace', 'siege'])('sends the dropped %s UUID after choosing an undeclared attacker', (defender) => {
    vi.useFakeTimers();
    const { hook, target, api } = setup();
    act(() => expect(hook.result.current.onDrop('bear', defender)).toBe(true));
    expect(api.sendPlayerUUID).toHaveBeenLastCalledWith('game', 'bear');
    target();
    act(() => vi.advanceTimersByTime(250));
    expect(api.sendPlayerUUID.mock.calls).toEqual([['game', 'bear'], ['game', defender]]);
  });

  it.each(['jace', 'siege'])('removes and redeclares a selected attacker before choosing %s', (defender) => {
    const { hook, declare, target, api } = setup();
    declare(true);
    act(() => expect(hook.result.current.onDrop('bear', defender)).toBe(true));
    declare();
    expect(api.sendPlayerUUID.mock.calls).toEqual([['game', 'bear'], ['game', 'bear']]);
    target();
    expect(api.sendPlayerUUID).toHaveBeenLastCalledWith('game', defender);
  });

  it.each(['jace', 'siege'])('answers when %s is the only legal defender offered', (defender) => {
    const { hook, target, api } = setup();
    act(() => { hook.result.current.onDrop('bear', defender); });
    target([defender]);
    expect(api.sendPlayerUUID).toHaveBeenLastCalledWith('game', defender);
  });

  it('clicking an attacker defaults to the one opponent', () => {
    vi.useFakeTimers();
    const { session, target, api } = setup();
    act(() => { session.click('bear'); });
    target();
    act(() => vi.advanceTimersByTime(250));
    expect(api.sendPlayerUUID).toHaveBeenLastCalledWith('game', 'them');
  });

  it('leaves a rejected drop for a manual choice even after another view arrives', () => {
    vi.useFakeTimers();
    const { hook, target, emit, view, api } = setup();
    act(() => { hook.result.current.onDrop('bear', 'siege'); });
    target(['them', 'jace']);
    emit('GAME_UPDATE', { ...view(), turn: 5 });
    act(() => vi.advanceTimersByTime(250));
    expect(api.sendPlayerUUID.mock.calls).toEqual([['game', 'bear']]);
    expect(hook.result.current.manualDefender).toBe(true);
  });
});

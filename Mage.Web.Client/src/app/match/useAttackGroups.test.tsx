import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useStore } from 'zustand';
import { GameSession } from '../../core/game/gameSession';
import type { EventBus } from '../../core/rpc/EventBus';
import type { GameView } from '../../protocol/generated/views';
import { useAttackGroups } from './useAttackGroups';

const TOKENS = ['t1', 't2', 't3', 't4', 't5', 't6'];

/** A server that declares or takes back the clicked token, asking "attack whom?" first when the opponent has a planeswalker. */
function setup({ planeswalker = false, refuse = [] as string[] } = {}) {
  const handlers = new Map<string, (data: unknown, event: { objectId: string }) => void>();
  const bus = { on(method: string, handler: (data: unknown, event: { objectId: string }) => void) {
    handlers.set(method, handler);
    return () => handlers.delete(method);
  } } as unknown as EventBus;
  const attacking = new Set<string>();
  let asking: string | null = null;
  const view = (): GameView => ({
    myPlayerId: 'me', step: 'DECLARE_ATTACKERS',
    players: [
      { playerId: 'me', name: 'Me', battlefield: Object.fromEntries(TOKENS.map((id) => [id, { id, name: 'Scute Swarm', power: '1', toughness: '1', isToken: true, cardTypes: ['CREATURE'], controllerId: 'me' }])) },
      { playerId: 'them', name: 'Them', life: 20, battlefield: planeswalker ? { jace: { id: 'jace', name: 'Jace', cardTypes: ['PLANESWALKER'] } } : {} },
    ],
    combat: attacking.size ? [{ defenderId: 'them', attackers: Object.fromEntries([...attacking].map((id) => [id, { id, controllerId: 'me' }])) }] : [],
  });
  const emit = (method: string, data: unknown) => handlers.get(method)?.(data, { objectId: 'game' });
  const declare = () => emit('GAME_SELECT', {
    message: 'Select attackers', gameView: view(), options: { possibleAttackers: TOKENS.filter((id) => !attacking.has(id)), specialButton: 'All attack' },
  });
  const api = {
    sendPlayerUUID: vi.fn(async (_game: string, id: string) => {
      // answered asynchronously, like the network
      await Promise.resolve();
      if (asking) {
        attacking.add(asking);
        asking = null;
      } else if (attacking.has(id)) {
        attacking.delete(id);
      } else if (!refuse.includes(id)) {
        if (planeswalker) {
          asking = id;
          emit('GAME_TARGET', { message: 'Select player, planeswalker, or battle to attack', gameView: view(), targets: ['them', 'jace'], flag: true });
          return true;
        }
        attacking.add(id);
      }
      declare();
      return true;
    }),
  };
  const session = new GameSession(api as never, bus, { gameId: 'game', playerId: 'me', mode: 'play' });
  act(() => declare());
  const hook = renderHook(() => useAttackGroups(session, useStore(session.store), true));
  const settle = () => act(async () => {
    for (let i = 0; i < 40; i++) await Promise.resolve();
  });
  return { hook, api, attacking, settle };
}

describe('attack with several of the same creature', () => {
  it('declares as many as asked, one click at a time, then takes some back', async () => {
    const { hook, attacking, settle } = setup();
    const [group] = hook.result.current.groups;
    expect(group.ready).toHaveLength(6);
    act(() => hook.result.current.want(group, 4));
    await settle();
    expect([...attacking]).toEqual(['t1', 't2', 't3', 't4']);
    expect(hook.result.current.busy).toBe(false);
    act(() => hook.result.current.want(hook.result.current.groups[0], 1));
    await settle();
    expect([...attacking]).toEqual(['t1']);
  });

  it('answers "attack whom?" with the opponent', async () => {
    const { hook, api, attacking, settle } = setup({ planeswalker: true });
    act(() => hook.result.current.want(hook.result.current.groups[0], 2));
    await settle();
    expect([...attacking]).toEqual(['t1', 't2']);
    expect(api.sendPlayerUUID.mock.calls.map(([, id]) => id)).toEqual(['t1', 'them', 't2', 'them']);
  });

  it('skips a creature the server refuses', async () => {
    const { hook, attacking, settle } = setup({ refuse: ['t2'] });
    act(() => hook.result.current.want(hook.result.current.groups[0], 3));
    await settle();
    expect([...attacking]).toEqual(['t1', 't3', 't4']);
    expect(hook.result.current.busy).toBe(false);
  });
});

import { act, renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useStore } from 'zustand';
import { GameSession } from '../../core/game/gameSession';
import type { EventBus } from '../../core/rpc/EventBus';
import type { GameView } from '../../protocol/generated/views';
import { useAttackGroups } from './useAttackGroups';

const TOKENS = ['t1', 't2', 't3', 't4', 't5', 't6'];

/** A server that declares or takes back the clicked token, asking "attack whom?" first when there is more than one defender. */
function setup({ planeswalker = false, multiplayer = false, refuse = [] as string[] } = {}) {
  const handlers = new Map<string, (data: unknown, event: { objectId: string }) => void>();
  const bus = { on(method: string, handler: (data: unknown, event: { objectId: string }) => void) {
    handlers.set(method, handler);
    return () => handlers.delete(method);
  } } as unknown as EventBus;
  // attacker id -> defender id
  const attacking = new Map<string, string>();
  let asking: string | null = null;
  const view = (): GameView => {
    const byDefender = new Map<string, string[]>();
    for (const [id, def] of attacking) {
      const list = byDefender.get(def) ?? [];
      list.push(id);
      byDefender.set(def, list);
    }
    return {
      myPlayerId: 'me', step: 'DECLARE_ATTACKERS',
      players: [
        { playerId: 'me', name: 'Me', battlefield: Object.fromEntries(TOKENS.map((id) => [id, { id, name: 'Scute Swarm', power: '1', toughness: '1', isToken: true, cardTypes: ['CREATURE'], controllerId: 'me' }])) },
        ...(multiplayer
          ? [
            { playerId: 'alice', name: 'Alice', life: 40, battlefield: {} },
            { playerId: 'bob', name: 'Bob', life: 40, battlefield: {} },
          ]
          : [
            { playerId: 'them', name: 'Them', life: 20, battlefield: planeswalker ? { jace: { id: 'jace', name: 'Jace', cardTypes: ['PLANESWALKER' as const] } } : {} },
          ]),
      ],
      combat: [...byDefender].map(([defenderId, ids]) => ({
        defenderId,
        attackers: Object.fromEntries(ids.map((id) => [id, { id, controllerId: 'me' }])),
      })),
    };
  };
  const emit = (method: string, data: unknown) => handlers.get(method)?.(data, { objectId: 'game' });
  const declare = () => emit('GAME_SELECT', {
    message: 'Select attackers', gameView: view(), options: { possibleAttackers: TOKENS.filter((id) => !attacking.has(id)), specialButton: 'All attack' },
  });
  const api = {
    sendPlayerUUID: vi.fn(async (_game: string, id: string) => {
      // answered asynchronously, like the network
      await Promise.resolve();
      if (asking) {
        attacking.set(asking, id);
        asking = null;
      } else if (attacking.has(id)) {
        attacking.delete(id);
      } else if (!refuse.includes(id)) {
        if (planeswalker || multiplayer) {
          asking = id;
          emit('GAME_TARGET', {
            message: 'Select player, planeswalker, or battle to attack',
            gameView: view(),
            targets: multiplayer ? ['alice', 'bob'] : ['them', 'jace'],
            flag: true,
          });
          return true;
        }
        attacking.set(id, 'them');
      }
      declare();
      return true;
    }),
    sendPlayerBoolean: vi.fn(async (_game: string, value: boolean) => {
      await Promise.resolve();
      if (!value && asking) {
        asking = null;
        declare();
      }
      return true;
    }),
  };
  const session = new GameSession(api as never, bus, { gameId: 'game', playerId: 'me', mode: 'play' });
  act(() => declare());
  const hook = renderHook(() => useAttackGroups(session, useStore(session.store), true));
  const settle = () => act(async () => {
    for (let i = 0; i < 60; i++) await Promise.resolve();
  });
  return { hook, api, session, attacking, settle };
}

describe('attack with several of the same creature', () => {
  it('declares as many as asked, one click at a time, then takes some back', async () => {
    const { hook, attacking, settle } = setup();
    const [group] = hook.result.current.groups;
    expect(group.ready).toHaveLength(6);
    act(() => hook.result.current.want(group, 4));
    await settle();
    expect([...attacking.keys()]).toEqual(['t1', 't2', 't3', 't4']);
    expect(hook.result.current.busy).toBe(false);
    act(() => hook.result.current.want(hook.result.current.groups[0], 1));
    await settle();
    expect([...attacking.keys()]).toEqual(['t1']);
  });

  it('answers "attack whom?" with the opponent', async () => {
    const { hook, api, attacking, settle } = setup({ planeswalker: true });
    act(() => hook.result.current.want(hook.result.current.groups[0], 2));
    await settle();
    expect([...attacking.keys()]).toEqual(['t1', 't2']);
    expect(api.sendPlayerUUID.mock.calls.map(([, id]) => id)).toEqual(['t1', 'them', 't2', 'them']);
  });

  it('skips a creature the server refuses', async () => {
    const { hook, attacking, settle } = setup({ refuse: ['t2'] });
    act(() => hook.result.current.want(hook.result.current.groups[0], 3));
    await settle();
    expect([...attacking.keys()]).toEqual(['t1', 't3', 't4']);
    expect(hook.result.current.busy).toBe(false);
  });

  it('in multiplayer, pauses for the first defender pick and sends the rest of the batch to that same player', async () => {
    const { hook, session, attacking, settle } = setup({ multiplayer: true });
    act(() => hook.result.current.want(hook.result.current.groups[0], 6));
    await settle();
    // paused waiting for the player to pick who to attack with the first token
    expect(session.getState().prompt?.kind).toBe('target');
    expect(hook.result.current.busy).toBe(true);
    // the player picks Bob: every remaining token automatically attacks Bob too
    act(() => { void session.respond({ type: 'uuid', id: 'bob' }); });
    await settle();
    expect([...attacking.entries()]).toEqual(TOKENS.map((id) => [id, 'bob']));
    expect(hook.result.current.busy).toBe(false);
  });

  it('in multiplayer, lets the player pre-pick defenders to split a token group across opponents', async () => {
    const { hook, attacking, settle } = setup({ multiplayer: true });
    act(() => hook.result.current.setDefender('alice'));
    act(() => hook.result.current.want(hook.result.current.groups[0], 2));
    await settle();
    expect([...attacking.entries()]).toEqual([['t1', 'alice'], ['t2', 'alice']]);

    act(() => hook.result.current.setDefender('bob'));
    expect(hook.result.current.groups[0].ready).toHaveLength(4);
    expect(hook.result.current.groups[0].attacking).toHaveLength(0);
    act(() => hook.result.current.want(hook.result.current.groups[0], 3));
    await settle();
    expect([...attacking.entries()]).toEqual([
      ['t1', 'alice'], ['t2', 'alice'],
      ['t3', 'bob'], ['t4', 'bob'], ['t5', 'bob'],
    ]);
  });

  it('in multiplayer, stops the batch if the player cancels the first defender pick', async () => {
    const { hook, session, attacking, settle } = setup({ multiplayer: true });
    act(() => hook.result.current.want(hook.result.current.groups[0], 6));
    await settle();
    expect(session.getState().prompt?.kind).toBe('target');
    act(() => { void session.respond({ type: 'boolean', value: false }); });
    await settle();
    expect(attacking.size).toBe(0);
    expect(hook.result.current.busy).toBe(false);
  });
});

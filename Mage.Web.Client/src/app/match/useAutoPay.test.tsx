// @vitest-environment happy-dom
import { cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { PermanentView } from '../../protocol/generated/views';
import type { PlayerBoard } from './boardModel';
import { useAutoPay } from './useAutoPay';

const plains = {
  id: 'plains-1',
  name: 'Plains',
  tapped: false,
  cardTypes: ['LAND'],
  subTypes: ['PLAINS'],
  rules: ['({T}: Add {W}.)'],
} as unknown as PermanentView;

const board: PlayerBoard = {
  isMe: true,
  player: { playerId: 'me', graveyard: {} },
  front: [],
  back: [{ key: 'plains', lead: plains, members: [plains] }],
} as unknown as PlayerBoard;

function payState(sourceId: string, stackRules: string[] = []): GameSessionState {
  return {
    playerId: 'me',
    awaitingServer: false,
    view: {
      myHand: {},
      stack: { [sourceId]: { id: sourceId, rules: stackRules } },
    },
    interaction: {
      mode: 'payMana',
      clickable: new Map([['plains-1', { type: 'uuid', id: 'plains-1' }]]),
      prompt: {
        kind: 'playMana',
        text: 'Pay {W}',
        message: `Pay {W}<div class='x'><font object_id='${sourceId}'>Spell</font></div>`,
        isX: false,
      },
    },
  } as unknown as GameSessionState;
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe('useAutoPay (#477)', () => {
  test('taps a plain land when enabled, and stays idle when disabled (e.g. Full Control)', () => {
    const click = vi.fn();
    const session = { click, respond: vi.fn() } as unknown as GameSession;
    const state = payState('spell-1');

    const { rerender } = renderHook(({ enabled }) => useAutoPay(session, state, board, enabled), {
      initialProps: { enabled: false },
    });
    vi.advanceTimersByTime(500);
    expect(click).not.toHaveBeenCalled();

    rerender({ enabled: true });
    vi.advanceTimersByTime(500);
    expect(click).toHaveBeenCalledWith('plains-1');
  });

  test('does not auto-tap for a Converge spell like Prismatic Ending', () => {
    const click = vi.fn();
    const session = { click, respond: vi.fn() } as unknown as GameSession;
    const state = payState('ending-1', [
      '<i>Converge</i> &mdash; Exile target nonland permanent if its mana value is less than or equal to the number of colors of mana spent to cast this spell.',
    ]);

    renderHook(() => useAutoPay(session, state, board, true, { afterX: true }));
    vi.advanceTimersByTime(500);
    expect(click).not.toHaveBeenCalled();
  });
});

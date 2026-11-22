// @vitest-environment happy-dom
import { act, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { Prompt } from '../../core/game/prompt';
import { useAutoAnswers } from '../stores/autoAnswers';
import { useAutoOrder } from './useAutoOrder';

function triggerPrompt(targets: string[]): Prompt {
  return {
    kind: 'target',
    text: 'Pick triggered ability (goes to the stack first)',
    required: true,
    targets,
    cards: Object.fromEntries(targets.map((id) => [id, { id, name: `Trigger ${id}` }])),
    options: { queryType: 'PICK_ABILITY' },
  } as unknown as Prompt;
}

function Harness({ session, state }: { session: GameSession; state: GameSessionState }) {
  useAutoOrder(session, state);
  return null;
}

describe('useAutoOrder', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useAutoAnswers.getState().forgetGame('g1');
    useAutoAnswers.getState().setAutoOrdering('g1', true);
  });

  afterEach(() => {
    vi.useRealTimers();
    useAutoAnswers.getState().forgetGame('g1');
  });

  test('does not re-send a rejected answer for the same prompt, and answers the next prompt after the delay', async () => {
    const respond = vi.fn(async () => {
      throw new Error('rejected');
    });
    const session = { respond } as unknown as GameSession;
    const prompt1 = triggerPrompt(['t1', 't2']);
    const baseState = { gameId: 'g1', prompt: prompt1, awaitingServer: false } as unknown as GameSessionState;

    const { rerender } = render(<Harness session={session} state={baseState} />);
    expect(respond).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(respond).toHaveBeenCalledTimes(1);
    expect(respond).toHaveBeenCalledWith({ type: 'uuid', id: 't1' });

    // Server refuses the answer: awaitingServer flips true -> false with the same prompt
    rerender(<Harness session={session} state={{ ...baseState, awaitingServer: true }} />);
    rerender(<Harness session={session} state={{ ...baseState, awaitingServer: false }} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(respond).toHaveBeenCalledTimes(1);

    // A new trigger-order prompt arrives and is answered once
    const prompt2 = triggerPrompt(['t2']);
    rerender(<Harness session={session} state={{ ...baseState, prompt: prompt2, awaitingServer: false }} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(respond).toHaveBeenCalledTimes(2);
    expect(respond).toHaveBeenLastCalledWith({ type: 'uuid', id: 't2' });
  });

  test('turns auto-ordering off when a non-trigger prompt arrives', () => {
    const respond = vi.fn(async () => undefined);
    const session = { respond } as unknown as GameSession;
    const otherPrompt = { kind: 'ask', text: 'Gain 1 life?', options: {} } as unknown as Prompt;
    render(<Harness session={session} state={{ gameId: 'g1', prompt: otherPrompt, awaitingServer: false } as unknown as GameSessionState} />);
    expect(useAutoAnswers.getState().autoOrdering['g1']).toBe(false);
    expect(respond).not.toHaveBeenCalled();
  });
});

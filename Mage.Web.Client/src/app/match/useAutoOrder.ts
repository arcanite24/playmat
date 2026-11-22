import { useEffect, useRef } from 'react';
import { autoOrderPick } from '../../core/game/autoAnswer';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { Prompt } from '../../core/game/prompt';
import { useAutoAnswers } from '../stores/autoAnswers';

const ORDER_DELAY_MS = 200;

/**
 * While "Order automatically" is on, answers each trigger-order question of the batch with the first trigger, and
 * turns itself off at the next other decision.
 */
export function useAutoOrder(session: GameSession, state: GameSessionState) {
  const { gameId, prompt, awaitingServer } = state;
  const on = useAutoAnswers((answers) => !!answers.autoOrdering[gameId]);
  const answered = useRef<Prompt | null>(null);
  useEffect(() => {
    if (!on || !prompt) {
      answered.current = null;
      return;
    }
    if (awaitingServer) return;
    const pick = autoOrderPick(prompt);
    if (!pick) {
      useAutoAnswers.getState().setAutoOrdering(gameId, false);
      return;
    }
    if (answered.current === prompt) return;
    const timer = setTimeout(() => {
      answered.current = prompt;
      void session.respond({ type: 'uuid', id: pick }).catch(() => undefined);
    }, ORDER_DELAY_MS);
    return () => clearTimeout(timer);
  }, [on, prompt, awaitingServer, session, gameId]);
  // the server forgets standing answers with the game
  useEffect(() => () => useAutoAnswers.getState().forgetGame(gameId), [gameId]);
}

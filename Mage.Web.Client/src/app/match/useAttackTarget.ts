import { useCallback, useEffect, useMemo, useRef } from 'react';
import { attackDropTargets, defaultDefender } from '../../core/game/attackTarget';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import { defenderChoice } from '../../core/game/multiplayer';
import type { Prompt } from '../../core/game/prompt';

const ANSWER_DELAY_MS = 200;

/**
 * Whom attackers go at: a creature dropped on a planeswalker or battle is sent to attack, and the server's "which
 * defender" question answered with where it was dropped. A creature already attacking is taken back and sent again,
 * so one that went at the player can be turned onto a planeswalker. With `defaultToPlayer`, the question for a clicked
 * attacker is answered with the one opponent. Returns the drop handler and what can be dropped on.
 */
export function useAttackTarget(session: GameSession, state: GameSessionState, myId: string | null, defaultToPlayer: boolean) {
  const pending = useRef<{ attackerId: string; defenderId: string; sentFor: Prompt | null; redeclare: boolean } | null>(null);
  const { prompt, awaitingServer, view } = state;
  const targets = useMemo(() => attackDropTargets(view, myId), [view, myId]);

  useEffect(() => {
    if (awaitingServer || !prompt) return;
    const current = pending.current;
    if (current && prompt === current.sentFor) return;
    const options = defenderChoice(prompt, view);
    if (current?.redeclare) {
      // taken back from its old target: declare it again, and answer the question that follows with the drop
      pending.current = { ...current, sentFor: prompt, redeclare: false };
      session.click(current.attackerId);
      return;
    }
    if (current) {
      pending.current = null;
      // a drop on something this creature can't attack leaves the question to the player
      if (options && prompt.kind === 'target' && prompt.targets.includes(current.defenderId)) {
        void session.respond({ type: 'uuid', id: current.defenderId }).catch(() => undefined);
      }
      return;
    }
    const player = defaultToPlayer && options ? defaultDefender(options) : null;
    if (!player) return;
    const timer = setTimeout(() => void session.respond({ type: 'uuid', id: player }).catch(() => undefined), ANSWER_DELAY_MS);
    return () => clearTimeout(timer);
  }, [session, prompt, awaitingServer, view, defaultToPlayer]);

  const onDrop = useCallback((attackerId: string, defenderId: string) => {
    const current = session.getState();
    if (current.awaitingServer || !current.interaction.clickable.has(attackerId)) return false;
    const redeclare = current.interaction.selected.has(attackerId);
    pending.current = { attackerId, defenderId, sentFor: current.prompt, redeclare };
    return session.click(attackerId);
  }, [session]);

  return { onDrop, targets };
}

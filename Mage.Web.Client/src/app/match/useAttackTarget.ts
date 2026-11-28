import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
  const handled = useRef<Prompt | null>(null);
  const [manualPrompt, setManualPrompt] = useState<Prompt | null>(null);
  const { prompt, awaitingServer, view } = state;
  const targets = useMemo(() => attackDropTargets(view, myId), [view, myId]);

  // A rejected drop must expose the server's choices even with the player-default setting on.
  useEffect(() => session.store.subscribe((next) => {
    const drop = pending.current;
    if (drop && !next.awaitingServer && next.prompt?.kind === 'target' && defenderChoice(next.prompt, next.view)
      && !next.prompt.targets.includes(drop.defenderId)) setManualPrompt(next.prompt);
  }), [session]);

  useEffect(() => {
    if (awaitingServer || !prompt || prompt === handled.current) return;
    const current = pending.current;
    if (current && prompt === current.sentFor) return;
    const options = defenderChoice(prompt, view);
    if (current?.redeclare && prompt.kind === 'declareAttackers') {
      // Only reselect after the server confirms removal, never during a nested cost or target choice.
      if (state.interaction.selected.has(current.attackerId) || !state.interaction.clickable.has(current.attackerId)) {
        pending.current = null;
        handled.current = prompt;
        return;
      }
      // taken back from its old target: declare it again, and answer the question that follows with the drop
      pending.current = { ...current, sentFor: prompt, redeclare: false };
      session.click(current.attackerId);
      return;
    }
    if (current) {
      pending.current = null;
      handled.current = prompt;
      // a drop on something this creature can't attack leaves the question to the player
      if (options && prompt.kind === 'target' && prompt.targets.includes(current.defenderId)) {
        void session.respond({ type: 'uuid', id: current.defenderId }).catch(() => undefined);
      }
      return;
    }
    const player = defaultToPlayer && options ? defaultDefender(options) : null;
    if (!player) return;
    const timer = setTimeout(() => {
      handled.current = prompt;
      void session.respond({ type: 'uuid', id: player }).catch(() => undefined);
    }, ANSWER_DELAY_MS);
    return () => clearTimeout(timer);
  }, [session, prompt, awaitingServer, view, defaultToPlayer, state.interaction]);

  const onDrop = useCallback((attackerId: string, defenderId: string) => {
    const current = session.getState();
    if (current.awaitingServer || current.prompt?.kind !== 'declareAttackers' || !targets.has(defenderId) || !current.interaction.clickable.has(attackerId)) return false;
    const redeclare = current.interaction.selected.has(attackerId);
    pending.current = { attackerId, defenderId, sentFor: current.prompt, redeclare };
    const sent = session.click(attackerId);
    if (!sent) pending.current = null;
    return sent;
  }, [session, targets]);

  return { onDrop, targets, manualDefender: manualPrompt === prompt };
}

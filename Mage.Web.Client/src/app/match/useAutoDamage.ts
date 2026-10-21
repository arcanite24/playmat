import { useEffect, useRef } from 'react';
import { autoAssignment } from '../../core/game/damageAssignment';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { Prompt } from '../../core/game/prompt';
import type { DamageSplit } from './useDamageSplit';

const ANSWER_DELAY_MS = 200;

/**
 * Assign combat damage for me: submits the lethal-first split when there is nothing to decide (see autoAssignment),
 * once per question; anything the player has started editing stays theirs.
 */
export function useAutoDamage(session: GameSession, state: GameSessionState, split: DamageSplit | null, options: { plain: boolean; trample: boolean }) {
  const answered = useRef<Prompt | null>(null);
  const { interaction, awaitingServer } = state;
  const prompt = interaction.prompt;
  const { plain, trample } = options;
  useEffect(() => {
    if (!split || awaitingServer || interaction.mode !== 'assignDamage' || !prompt || answered.current === prompt) return;
    if (split.assignment.trample ? !trample : !plain) return;
    const answer = autoAssignment(split.assignment, { trample });
    // the split on the board was edited: the player is deciding
    if (!answer || answer !== split.values.join(' ')) return;
    const timer = setTimeout(() => {
      answered.current = prompt;
      void session.respond({ type: 'string', value: answer }).catch(() => undefined);
    }, ANSWER_DELAY_MS);
    return () => clearTimeout(timer);
  }, [session, split, awaitingServer, interaction.mode, prompt, plain, trample]);
}

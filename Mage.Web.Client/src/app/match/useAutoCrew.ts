import { useEffect, useRef } from 'react';
import { autoCrewPick, crewCandidate, crewNeed, type CrewCandidate } from '../../core/game/autoCrew';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { PlayerBoard } from './boardModel';

const TAP_DELAY_MS = 180;

/**
 * Crews a Vehicle for the player when only one group of creatures makes sense: taps them one by one as the server
 * asks, then finishes the choice. A choice the player has started themself, or one with real options, is left alone.
 */
export function useAutoCrew(session: GameSession, state: GameSessionState, me: PlayerBoard | null, enabled: boolean) {
  const plan = useRef<string[] | null>(null);
  const { interaction, awaitingServer } = state;
  const prompt = interaction.prompt;

  useEffect(() => {
    const crew = prompt?.kind === 'target' ? crewNeed(prompt.text) : null;
    if (!enabled || !me || !crew || prompt?.kind !== 'target' || interaction.mode !== 'target') {
      plan.current = null;
      return;
    }
    if (awaitingServer) return;
    const chosen = prompt.chosen;
    if (!plan.current) {
      // only a fresh choice: one the player has begun is theirs
      if (chosen.length > 0 || crew.selected > 0) return;
      const permanents = new Map(me.front.concat(me.back).flatMap((group) => group.members).map((permanent) => [permanent.id!, permanent]));
      const candidates: CrewCandidate[] = [];
      for (const id of prompt.targets) {
        const permanent = permanents.get(id);
        const candidate = permanent ? crewCandidate(permanent) : null;
        // something we can't weigh (a creature that crews by toughness, a target off our board): the player decides
        if (!candidate) return;
        candidates.push(candidate);
      }
      const pick = autoCrewPick(crew.need, candidates);
      if (!pick) return;
      plan.current = pick;
    }
    const planned = plan.current;
    // the player picked something else meanwhile: hand the choice back
    if (chosen.some((id) => !planned.includes(id))) {
      plan.current = null;
      return;
    }
    const next = planned.find((id) => !chosen.includes(id));
    const timer = setTimeout(() => {
      if (next) session.click(next);
      else if (crew.selected >= crew.need) void session.respond({ type: 'uuid', id: null }).catch(() => undefined);
    }, TAP_DELAY_MS);
    return () => clearTimeout(timer);
  }, [enabled, me, awaitingServer, interaction, prompt, session]);
}

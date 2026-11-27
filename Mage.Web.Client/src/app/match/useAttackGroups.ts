import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { attackerDefender, attackGroups, nextAttackClick, type AttackGroup } from '../../core/game/attackGroups';
import { defaultDefender } from '../../core/game/attackTarget';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import { defenderChoice, possibleDefenders, type DefenderOption } from '../../core/game/multiplayer';
import type { Prompt } from '../../core/game/prompt';

const NONE: ReadonlyMap<string, number> = new Map();

/**
 * "Attack with N of these": the player sets how many of a group of identical creatures attack, and this clicks them
 * one at a time as the server asks, answering "attack whom?" with the chosen defender (or the sole opponent). In
 * multiplayer, if no defender was pre-selected, the first creature pauses for the player's defender choice and then
 * sends the rest of the batch to that same defender. Anything else the server asks on the way (a cost to attack) stops
 * it and is left to the player; a creature the server won't declare is skipped.
 */
export function useAttackGroups(session: GameSession, state: GameSessionState, enabled: boolean) {
  const [wanted, setWanted] = useState<ReadonlyMap<string, number>>(NONE);
  const [selectedDefender, setSelectedDefender] = useState<string | null>(null);
  const plan = useRef<ReadonlyMap<string, number>>(NONE);
  const defenderRef = useRef<string | null>(null);
  const batchDefender = useRef<string | null>(null);
  const awaitingDefender = useRef(false);
  const refused = useRef(new Set<string>());
  const last = useRef<{ id: string; wasAttacking: boolean; prompt: Prompt } | null>(null);
  const driving = useRef(false);
  const on = useRef(enabled);
  const { view, interaction } = state;
  const defenders = useMemo<DefenderOption[]>(
    () => (enabled ? possibleDefenders(view, view?.myPlayerId ?? null) : []),
    [enabled, view],
  );
  // drop a selected defender if it left the game / battlefield (or only one defender remains)
  const defender = defenders.length > 1 && selectedDefender && defenders.some((option) => option.id === selectedDefender)
    ? selectedDefender
    : null;
  defenderRef.current = defender;

  const live = useMemo(
    () => (enabled ? attackGroups(view, interaction.prompt, undefined, defender) : []),
    [enabled, view, interaction.prompt, defender],
  );
  // while the clicks are being made, a question in between ("attack whom?") isn't the declare-attackers prompt: the
  // panel keeps the groups it had, so it doesn't vanish under the player mid-declaration
  const kept = useRef<AttackGroup[]>([]);
  if (live.length > 0 || wanted.size === 0) kept.current = live;
  const groups = live.length > 0 ? live : kept.current;

  const stop = useCallback(() => {
    plan.current = NONE;
    batchDefender.current = null;
    awaitingDefender.current = false;
    refused.current.clear();
    last.current = null;
    setWanted(NONE);
  }, []);

  // one step: called on every change of the session, and when the player changes a count
  const drive = useCallback(() => {
    if (plan.current.size === 0 || driving.current) return;
    const { prompt, awaitingServer, view: current, interaction: now } = session.getState();
    // the interaction (what's clickable) is derived from the prompt a moment later: wait for it
    if (awaitingServer || !prompt || now.prompt !== prompt) return;
    if (!on.current) {
      stop();
      return;
    }
    // a click changes the session at once; its own notifications mustn't click again
    driving.current = true;
    try {
      if (prompt.kind === 'declareAttackers') {
        const activeDefender = defenderRef.current;
        const groupsNow = attackGroups(current, prompt, undefined, activeDefender);
        const isAttacking = (id: string) => groupsNow.some((group) => group.attacking.includes(id));
        const clicked = last.current;
        if (clicked && clicked.prompt !== prompt) {
          if (awaitingDefender.current) {
            awaitingDefender.current = false;
            const chosen = attackerDefender(current, clicked.id);
            if (!chosen) {
              // the player cancelled the defender choice: stop the batch
              stop();
              return;
            }
            batchDefender.current = chosen;
          } else if (isAttacking(clicked.id) === clicked.wasAttacking) {
            // the creature clicked last is where it was: the server didn't take it, so it's skipped from now on
            refused.current.add(clicked.id);
          }
          last.current = null;
        }
        if (last.current) return;
        const next = nextAttackClick(groupsNow, plan.current, refused.current);
        if (!next) {
          stop();
          return;
        }
        last.current = { id: next, wasAttacking: isAttacking(next), prompt };
        if (!session.click(next)) {
          refused.current.add(next);
          last.current = null;
        }
        return;
      }
      const options = defenderChoice(prompt, current);
      if (options) {
        const target = batchDefender.current ?? defenderRef.current;
        const answer = (target && options.some((option) => option.id === target)) ? target : defaultDefender(options);
        if (answer) {
          void session.respond({ type: 'uuid', id: answer }).catch(() => undefined);
          return;
        }
        // multiplayer with no defender chosen yet: pause and let the player pick via DefenderPicker or seat click,
        // then remember that defender for the rest of the batch once declareAttackers returns
        awaitingDefender.current = true;
        return;
      }
      stop();
    } finally {
      driving.current = false;
    }
  }, [session, stop]);

  useEffect(() => {
    on.current = enabled;
  }, [enabled]);
  useEffect(() => session.store.subscribe(() => drive()), [session, drive]);

  const setDefender = useCallback((id: string | null) => {
    stop();
    defenderRef.current = id;
    setSelectedDefender(id);
  }, [stop]);

  const want = useCallback((group: AttackGroup, count: number) => {
    refused.current.clear();
    batchDefender.current = defenderRef.current;
    awaitingDefender.current = false;
    plan.current = new Map(plan.current).set(group.key, count);
    setWanted(plan.current);
    drive();
  }, [drive]);

  // how many clicks are left
  const remaining = groups.reduce((sum, group) => {
    const target = wanted.get(group.key);
    return target === undefined ? sum : sum + Math.abs(target - group.attacking.length);
  }, 0);

  return { groups, wanted, want, stop, busy: wanted.size > 0, remaining, defenders, defender, setDefender };
}

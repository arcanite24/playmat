import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { attackGroups, nextAttackClick, type AttackGroup } from '../../core/game/attackGroups';
import { defaultDefender } from '../../core/game/attackTarget';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import { defenderChoice } from '../../core/game/multiplayer';
import type { Prompt } from '../../core/game/prompt';

const NONE: ReadonlyMap<string, number> = new Map();

/**
 * "Attack with N of these": the player sets how many of a group of identical creatures attack, and this clicks them
 * one at a time as the server asks, answering "attack whom?" with the one opponent. Anything else the server asks on
 * the way (a cost to attack, a choice among several opponents) stops it and is left to the player; a creature the
 * server won't declare is skipped.
 */
export function useAttackGroups(session: GameSession, state: GameSessionState, enabled: boolean) {
  const [wanted, setWanted] = useState<ReadonlyMap<string, number>>(NONE);
  const plan = useRef<ReadonlyMap<string, number>>(NONE);
  const refused = useRef(new Set<string>());
  const last = useRef<{ id: string; wasAttacking: boolean; prompt: Prompt } | null>(null);
  const driving = useRef(false);
  const on = useRef(enabled);
  const { view, interaction } = state;
  const live = useMemo(() => (enabled ? attackGroups(view, interaction.prompt) : []), [enabled, view, interaction.prompt]);
  // while the clicks are being made, a question in between ("attack whom?") isn't the declare-attackers prompt: the
  // panel keeps the groups it had, so it doesn't vanish under the player mid-declaration
  const kept = useRef<AttackGroup[]>([]);
  if (live.length > 0 || wanted.size === 0) kept.current = live;
  const groups = live.length > 0 ? live : kept.current;

  const stop = useCallback(() => {
    plan.current = NONE;
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
        const groupsNow = attackGroups(current, prompt);
        const isAttacking = (id: string) => groupsNow.some((group) => group.attacking.includes(id));
        // the creature clicked last is where it was: the server didn't take it, so it's skipped from now on
        const clicked = last.current;
        if (clicked && clicked.prompt !== prompt) {
          if (isAttacking(clicked.id) === clicked.wasAttacking) refused.current.add(clicked.id);
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
      const player = options ? defaultDefender(options) : null;
      if (player) {
        void session.respond({ type: 'uuid', id: player }).catch(() => undefined);
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

  const want = useCallback((group: AttackGroup, count: number) => {
    refused.current.clear();
    plan.current = new Map(plan.current).set(group.key, count);
    setWanted(plan.current);
    drive();
  }, [drive]);

  // how many clicks are left
  const remaining = groups.reduce((sum, group) => {
    const target = wanted.get(group.key);
    return target === undefined ? sum : sum + Math.abs(target - group.attacking.length);
  }, 0);

  return { groups, wanted, want, stop, busy: wanted.size > 0, remaining };
}

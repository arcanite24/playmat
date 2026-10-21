import { useEffect, useRef } from 'react';
import { crewNeed } from '../../core/game/autoCrew';
import { identicalPick, proliferatePicks, singleChoice } from '../../core/game/autoChoice';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';

const ANSWER_DELAY_MS = 200;

/** A fingerprint of the question on screen, to answer each one once. */
function promptKey(state: GameSessionState): string | null {
  const prompt = state.interaction.prompt;
  if (!prompt) return null;
  const chosen = prompt.kind === 'target' ? prompt.chosen.join() : '';
  const options = prompt.kind === 'target' ? prompt.targets.join() : prompt.kind === 'chooseAbility' ? prompt.choices.map((choice) => choice.id).join() : '';
  return `${prompt.kind}|${prompt.text}|${options}|${chosen}`;
}

/** Skip one-option menus: a menu with a single option that must be taken takes it. */
export function useSkipSingleChoice(session: GameSession, state: GameSessionState, enabled: boolean) {
  const answered = useRef<string | null>(null);
  const { interaction, awaitingServer } = state;
  useEffect(() => {
    const command = enabled && !awaitingServer ? singleChoice(interaction.prompt) : null;
    const key = promptKey(state);
    if (!command || key === answered.current) return;
    const timer = setTimeout(() => {
      answered.current = key;
      void session.respond(command).catch(() => undefined);
    }, ANSWER_DELAY_MS);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the key is read from the interaction
  }, [enabled, awaitingServer, interaction, session]);
}

/**
 * Pick among identical options: when everything left to choose from is the same, chooses for the player, one object
 * at a time as the server asks. A choice the player has started themself stays theirs.
 */
export function usePickIdentical(session: GameSession, state: GameSessionState, enabled: boolean) {
  const picked = useRef<string[]>([]);
  const acted = useRef<string | null>(null);
  const { interaction, awaitingServer, view } = state;
  const prompt = interaction.prompt;
  useEffect(() => {
    if (prompt?.kind !== 'target' || (interaction.mode !== 'target' && interaction.mode !== 'pickCards')) {
      picked.current = [];
      acted.current = null;
      return;
    }
    if (!enabled || awaitingServer || crewNeed(prompt.text)) return;
    // the player chose something themself: the rest is theirs too
    if (prompt.chosen.some((id) => !picked.current.includes(id))) return;
    const key = promptKey(state);
    // the same question again after a pick: the pick didn't take, so don't insist
    if (key === acted.current) return;
    const next = identicalPick(prompt, view);
    if (!next) return;
    const timer = setTimeout(() => {
      acted.current = key;
      picked.current = [...picked.current, next];
      session.click(next);
    }, ANSWER_DELAY_MS);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- the key is read from the interaction
  }, [enabled, awaitingServer, interaction, prompt, view, session]);
}

/**
 * Smart preselection: proliferate starts with the obvious answers ticked (see proliferatePicks), each clicked once;
 * the player adds or removes any and confirms.
 */
export function useSmartPreselect(session: GameSession, state: GameSessionState, myId: string | null, enabled: boolean) {
  const plan = useRef<{ ids: string[]; clicked: string[] } | null>(null);
  const { interaction, awaitingServer, view } = state;
  const prompt = interaction.prompt;
  useEffect(() => {
    if (prompt?.kind !== 'target' || interaction.mode !== 'target') {
      plan.current = null;
      return;
    }
    if (!enabled || awaitingServer) return;
    if (!plan.current) {
      if (prompt.chosen.length > 0) return;
      plan.current = { ids: proliferatePicks(prompt, view, myId), clicked: [] };
    }
    const current = plan.current;
    // the player took over (chose or removed something): leave the rest to them
    if (prompt.chosen.length !== current.clicked.length || prompt.chosen.some((id) => !current.clicked.includes(id))) {
      current.ids = [];
      return;
    }
    const next = current.ids.find((id) => !current.clicked.includes(id) && prompt.targets.includes(id));
    if (!next) return;
    const timer = setTimeout(() => {
      current.clicked.push(next);
      session.click(next);
    }, ANSWER_DELAY_MS);
    return () => clearTimeout(timer);
  }, [enabled, awaitingServer, interaction, prompt, view, myId, session]);
}

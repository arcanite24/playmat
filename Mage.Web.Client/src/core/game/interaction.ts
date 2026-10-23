import type { GameView, ManaType, PlayerAction } from '../../protocol/generated/views';
import { assignmentAnswer, lethalFirst, parseDamageAssignment } from './damageAssignment';
import { poolId, poolTypes } from './payment';
import type { Prompt } from './prompt';

/** An answer to the server, independent of how the UI produced it. */
export type Command =
  | { type: 'uuid'; id: string | null }
  | { type: 'boolean'; value: boolean }
  | { type: 'string'; value: string }
  | { type: 'integer'; value: number }
  | { type: 'manaType'; manaType: ManaType }
  | { type: 'action'; action: PlayerAction; data?: string | number | boolean | null };

export type InteractionMode =
  /** nothing to answer: opponent's priority, resolving, waiting */
  | 'waiting'
  | 'priority'
  | 'mulligan'
  | 'declareAttackers'
  | 'declareBlockers'
  /** choose objects or players on the board */
  | 'target'
  /** choose among cards shown in a picker (library search, cards outside the battlefield) */
  | 'pickCards'
  | 'payMana'
  /** divide an attacker's combat damage among its blockers, on the blockers themselves */
  | 'assignDamage'
  /** answered with buttons in the prompt bar */
  | 'question'
  /** needs a dedicated panel: modes, piles, numbers, choice lists */
  | 'panel';

export type ButtonTone = 'primary' | 'secondary' | 'danger' | 'attack';

export interface PromptButton {
  label: string;
  command: Command;
  tone: ButtonTone;
  /** keyboard shortcut hint, e.g. "Space" */
  shortcut?: string;
}

export interface Interaction {
  mode: InteractionMode;
  prompt: Prompt | null;
  /** short instruction for the prompt bar */
  headline: string;
  /** ids (cards, permanents, players, stack objects) the player may click, and what a click sends */
  clickable: ReadonlyMap<string, Command>;
  /** ids currently chosen in this prompt (targets, attackers, blockers) */
  selected: ReadonlySet<string>;
  /** the big contextual button (Arena's bottom-right button) */
  mainButton: PromptButton | null;
  secondaryButtons: PromptButton[];
}

const PASS: Command = { type: 'boolean', value: false };
const EMPTY_MAP: ReadonlyMap<string, Command> = new Map();
const EMPTY_SET: ReadonlySet<string> = new Set();

function uuid(id: string | null): Command {
  return { type: 'uuid', id };
}

function clickMap(ids: Iterable<string>): Map<string, Command> {
  const map = new Map<string, Command>();
  for (const id of ids) map.set(id, uuid(id));
  return map;
}

/** Objects the player can play or activate right now (from the server's playable list). */
export function playableIds(view: GameView | null | undefined): string[] {
  return Object.keys(view?.canPlayObjects?.objects ?? {});
}

function myPlayer(view: GameView | null | undefined) {
  return view?.players?.find((player) => player.playerId === view.myPlayerId);
}

function isMyTurn(view: GameView | null | undefined): boolean {
  return !!view?.myPlayerId && view.activePlayerId === view.myPlayerId;
}

/** Label for passing priority, following Arena's wording. */
export function passLabel(view: GameView | null | undefined): string {
  if (!view) return 'Pass';
  if (view.stack && Object.keys(view.stack).length > 0) return 'Resolve';
  if (!isMyTurn(view)) return 'Pass';
  switch (view.step) {
    case 'UPKEEP':
    case 'DRAW':
      return 'Next';
    case 'PRECOMBAT_MAIN':
      return 'To Combat';
    case 'BEGIN_COMBAT':
    case 'DECLARE_ATTACKERS':
    case 'DECLARE_BLOCKERS':
    case 'FIRST_COMBAT_DAMAGE':
    case 'COMBAT_DAMAGE':
    case 'END_COMBAT':
      return 'Next';
    case 'POSTCOMBAT_MAIN':
    case 'END_TURN':
    case 'CLEANUP':
      return 'End Turn';
    default:
      return 'Next';
  }
}

export function myAttackers(view: GameView | null | undefined): string[] {
  const me = view?.myPlayerId;
  return (view?.combat ?? []).flatMap((group) =>
    Object.entries(group.attackers ?? {})
      .filter(([, card]) => !me || card.controllerId === undefined || card.controllerId === me)
      .map(([id]) => id));
}

export function myBlockers(view: GameView | null | undefined): string[] {
  const me = view?.myPlayerId;
  return (view?.combat ?? []).flatMap((group) =>
    Object.entries(group.blockers ?? {})
      .filter(([, card]) => !me || card.controllerId === undefined || card.controllerId === me)
      .map(([id]) => id));
}

/**
 * Decides what the board does for the current prompt. Pure: the same view and prompt always give the same answer.
 */
export function deriveInteraction(view: GameView | null | undefined, prompt: Prompt | null): Interaction {
  if (!prompt) {
    return {
      mode: 'waiting',
      prompt: null,
      headline: view?.priorityPlayerName && view.priorityPlayerName !== myPlayer(view)?.name
        ? `Waiting for ${view.priorityPlayerName}`
        : 'Waiting',
      clickable: EMPTY_MAP,
      selected: EMPTY_SET,
      mainButton: null,
      secondaryButtons: [],
    };
  }

  switch (prompt.kind) {
    case 'priority':
      return {
        mode: 'priority',
        prompt,
        headline: prompt.text,
        clickable: clickMap(playableIds(view)),
        selected: EMPTY_SET,
        mainButton: { label: passLabel(view), command: PASS, tone: 'primary', shortcut: 'Space' },
        secondaryButtons: [],
      };

    case 'declareAttackers': {
      const selected = new Set(myAttackers(view));
      const noAttacks: PromptButton = { label: 'No Attacks', command: { type: 'boolean', value: true }, tone: 'secondary' };
      // with nothing chosen yet, the big button (and Space) attacks with everything, as on Arena: skipping the attack
      // takes a deliberate click on the smaller "No Attacks"
      const allAttack = !!prompt.allAttackLabel && prompt.possibleAttackers.length > 0;
      const mainButton: PromptButton = selected.size > 0
        ? { label: `Attack (${selected.size})`, command: { type: 'boolean', value: true }, tone: 'attack', shortcut: 'Space' }
        : allAttack
          ? { label: prompt.allAttackLabel!, command: { type: 'string', value: 'special' }, tone: 'attack', shortcut: 'Space' }
          : { ...noAttacks, tone: 'primary', shortcut: 'Space' };
      return {
        mode: 'declareAttackers',
        prompt,
        headline: selected.size > 0 ? 'Choose attackers' : prompt.text,
        clickable: clickMap(new Set([...prompt.possibleAttackers, ...selected])),
        selected,
        mainButton,
        // once some are chosen, "No Attacks" would confirm them: un-choosing is done on the cards
        secondaryButtons: !allAttack ? [] : selected.size > 0
          ? [{ label: prompt.allAttackLabel!, command: { type: 'string', value: 'special' }, tone: 'attack' }]
          : [noAttacks],
      };
    }

    case 'declareBlockers': {
      const selected = new Set(myBlockers(view));
      return {
        mode: 'declareBlockers',
        prompt,
        headline: selected.size > 0 ? 'Choose blockers' : prompt.text,
        clickable: clickMap(new Set([...prompt.possibleBlockers, ...selected])),
        selected,
        mainButton: {
          label: selected.size === 0 ? 'No Blocks' : `Block (${selected.size})`,
          command: { type: 'boolean', value: true },
          tone: 'primary',
          shortcut: 'Space',
        },
        secondaryButtons: [],
      };
    }

    case 'target': {
      const selected = new Set(prompt.chosen);
      // an optional choice (always the case while casting or activating) can be abandoned: answering nothing
      // cancels the spell or ability, or skips an optional effect
      const canAbandon = !prompt.required && selected.size === 0;
      const canFinish = selected.size > 0 && (!prompt.required || prompt.doneLabel !== null);
      // choosing no card is a normal answer off the board (a library search may find nothing that matches)
      const pickNone = canAbandon && prompt.cards !== null;
      const mainButton: PromptButton | null = canFinish || pickNone
        ? { label: prompt.doneLabel ?? 'Done', command: uuid(null), tone: 'primary', shortcut: 'Space' }
        : null;
      const secondaryButtons: PromptButton[] = canAbandon && !pickNone ? [{ label: 'Cancel', command: uuid(null), tone: 'danger' }] : [];
      return {
        mode: prompt.cards ? 'pickCards' : 'target',
        prompt,
        headline: prompt.text,
        clickable: clickMap(new Set([...prompt.targets, ...selected])),
        selected,
        mainButton,
        secondaryButtons,
      };
    }

    case 'ask':
      return {
        mode: prompt.isMulligan ? 'mulligan' : 'question',
        prompt,
        headline: prompt.text,
        clickable: EMPTY_MAP,
        selected: EMPTY_SET,
        // the main button is the negative answer; the affirmative stays secondary
        mainButton: { label: prompt.noLabel, command: { type: 'boolean', value: false }, tone: 'primary' },
        secondaryButtons: [{ label: prompt.yesLabel, command: { type: 'boolean', value: true }, tone: 'secondary' }],
      };

    case 'playMana': {
      const secondary: PromptButton[] = [];
      if (prompt.specialLabel) {
        secondary.push({ label: prompt.specialLabel, command: { type: 'string', value: 'special' }, tone: 'secondary' });
      }
      // floating mana pays too: each kind in the pool is clicked like a mana source
      const clickable = clickMap(playableIds(view));
      for (const manaType of poolTypes(myPlayer(view))) clickable.set(poolId(manaType), { type: 'manaType', manaType });
      return {
        mode: 'payMana',
        prompt,
        headline: prompt.text,
        clickable,
        selected: EMPTY_SET,
        // X costs finish with any boolean; for normal costs any boolean cancels the spell
        mainButton: prompt.isX
          ? { label: 'Done', command: { type: 'boolean', value: true }, tone: 'primary', shortcut: 'Space' }
          : null,
        secondaryButtons: [
          ...secondary,
          { label: 'Cancel', command: { type: 'boolean', value: false }, tone: 'danger' },
        ],
      };
    }

    case 'multiAmount': {
      const assignment = parseDamageAssignment(prompt, view);
      if (!assignment) return panel(prompt);
      return {
        mode: 'assignDamage',
        prompt,
        headline: assignment.sourceName ? `Assign ${assignment.sourceName}'s combat damage` : 'Assign combat damage',
        clickable: EMPTY_MAP,
        selected: EMPTY_SET,
        // the board edits the split; this answer is the lethal-first default it starts from
        mainButton: { label: 'Assign damage', command: { type: 'string', value: assignmentAnswer(lethalFirst(assignment)) }, tone: 'primary', shortcut: 'Space' },
        secondaryButtons: [],
      };
    }

    case 'chooseAbility':
    case 'choosePile':
    case 'chooseChoice':
    case 'amount':
      return panel(prompt);
  }
}

function panel(prompt: Prompt): Interaction {
  return {
    mode: 'panel',
    prompt,
    headline: prompt.text || 'Make a choice',
    clickable: EMPTY_MAP,
    selected: EMPTY_SET,
    mainButton: null,
    secondaryButtons: [],
  };
}

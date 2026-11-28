import { describe, expect, test } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { deriveInteraction, passLabel } from './interaction';
import { parsePrompt } from './prompt';
import { structuralShare } from './structuralShare';

const ME = 'me';
const OPPONENT = 'opponent';

function view(overrides: Partial<GameView> = {}): GameView {
  return {
    myPlayerId: ME,
    activePlayerId: ME,
    step: 'PRECOMBAT_MAIN',
    turn: 3,
    stack: {},
    players: [
      { playerId: ME, name: 'Me', life: 20 },
      { playerId: OPPONENT, name: 'Opponent', life: 20 },
    ],
    canPlayObjects: { objects: { 'land-1': {}, 'bolt-1': {} } },
    ...overrides,
  };
}

describe('prompt parsing', () => {
  test('a select without options is the priority prompt', () => {
    expect(parsePrompt('GAME_SELECT', { message: 'Play spells and abilities' }).kind).toBe('priority');
  });

  test('attack and block prompts are recognized by their options', () => {
    const attack = parsePrompt('GAME_SELECT', {
      message: 'Select attackers',
      options: { possibleAttackers: ['a', 'b'], specialButton: 'All attack' } as never,
    });
    expect(attack).toMatchObject({ kind: 'declareAttackers', possibleAttackers: ['a', 'b'], allAttackLabel: 'All attack' });
    expect(parsePrompt('GAME_SELECT', { message: 'Select blockers', options: { possibleBlockers: ['c'] } as never }))
      .toMatchObject({ kind: 'declareBlockers', possibleBlockers: ['c'] });
  });

  test('the mulligan question is detected from its buttons', () => {
    const prompt = parsePrompt('GAME_ASK', {
      message: 'Mulligan <font color=yellow>down to 6 cards</font>?',
      options: { 'UI.left.btn.text': 'Mulligan', 'UI.right.btn.text': 'Keep' } as never,
    });
    expect(prompt).toMatchObject({ kind: 'ask', isMulligan: true, yesLabel: 'Mulligan', noLabel: 'Keep', text: 'Mulligan down to 6 cards?' });
  });

  test('target prompts carry valid targets, chosen ones and off-board cards', () => {
    const prompt = parsePrompt('GAME_TARGET', {
      message: 'Select a target',
      targets: [OPPONENT, 'creature-1'],
      flag: true,
      options: { chosenTargets: ['creature-1'], 'UI.right.btn.text': 'Done' } as never,
    });
    expect(prompt).toMatchObject({ kind: 'target', required: true, chosen: ['creature-1'], doneLabel: 'Done', cards: null });
  });

  test('an optional trigger\'s target prompt is marked by the server', () => {
    const message = { message: 'Select target artifact or enchantment', targets: ['artifact-1'], flag: true };
    expect(parsePrompt('GAME_TARGET', message)).toMatchObject({ kind: 'target', optionalTrigger: false });
    expect(parsePrompt('GAME_TARGET', { ...message, options: { optionalTrigger: true } as never }))
      .toMatchObject({ kind: 'target', optionalTrigger: true, required: true });
  });
});

describe('interaction model', () => {
  test('an optional trigger\'s target prompt says the "may" comes when it resolves', () => {
    const prompt = parsePrompt('GAME_TARGET', {
      message: 'Select target artifact or enchantment',
      targets: ['artifact-1'],
      flag: true,
      options: { optionalTrigger: true } as never,
    });
    const interaction = deriveInteraction(view(), prompt);
    expect(interaction.mode).toBe('target');
    expect(interaction.headline).toBe('Select target artifact or enchantment. Optional: you can still decline when it resolves.');
    // the target itself is still required: no cancel button
    expect(interaction.secondaryButtons).toEqual([]);
    expect(deriveInteraction(view(), parsePrompt('GAME_TARGET', { message: 'Select target creature', targets: ['c'], flag: true })).headline)
      .toBe('Select target creature');
  });

  test('priority: playable objects are clickable and the main button passes', () => {
    const interaction = deriveInteraction(view(), parsePrompt('GAME_SELECT', { message: 'Play spells and abilities' }));
    expect(interaction.mode).toBe('priority');
    expect([...interaction.clickable.keys()]).toEqual(['land-1', 'bolt-1']);
    expect(interaction.clickable.get('bolt-1')).toEqual({ type: 'uuid', id: 'bolt-1' });
    expect(interaction.mainButton).toMatchObject({ label: 'To Combat', command: { type: 'boolean', value: false } });
  });

  test('main button follows Arena wording', () => {
    expect(passLabel(view({ stack: { s: { name: 'Bolt' } } }))).toBe('Resolve');
    expect(passLabel(view({ activePlayerId: OPPONENT }))).toBe('Pass');
    expect(passLabel(view({ step: 'POSTCOMBAT_MAIN' }))).toBe('End Turn');
  });

  test('attacks: with none chosen, Space attacks with all and "No Attacks" takes a click', () => {
    const prompt = parsePrompt('GAME_SELECT', {
      message: 'Select attackers',
      options: { possibleAttackers: ['bear'], specialButton: 'All attack' } as never,
    });
    const idle = deriveInteraction(view(), prompt);
    expect(idle.mainButton).toMatchObject({ label: 'All attack', command: { type: 'string', value: 'special' }, shortcut: 'Space' });
    expect(idle.secondaryButtons[0]).toMatchObject({ label: 'No Attacks', command: { type: 'boolean', value: true } });

    const attacking = deriveInteraction(view({
      combat: [{ defenderId: OPPONENT, attackers: { bear: { name: 'Bear', controllerId: ME } } }],
    }), prompt);
    expect(attacking.selected.has('bear')).toBe(true);
    expect(attacking.mainButton?.label).toBe('Attack (1)');
    expect(attacking.secondaryButtons.map((button) => button.label)).toEqual(['All attack']);
  });

  test('targets: players are clickable on the board, finishing needs enough targets', () => {
    const required = deriveInteraction(view(), parsePrompt('GAME_TARGET', {
      message: 'Choose any target', targets: [OPPONENT, 'bear'], flag: true,
    }));
    expect(required.mode).toBe('target');
    expect(required.clickable.has(OPPONENT)).toBe(true);
    expect(required.mainButton).toBeNull();

    const optional = deriveInteraction(view(), parsePrompt('GAME_TARGET', {
      message: 'Choose up to one target', targets: ['bear'], flag: false,
    }));
    // nothing chosen yet on an optional choice (e.g. while casting): Cancel abandons it
    expect(optional.mainButton).toBeNull();
    expect(optional.secondaryButtons).toEqual([{ label: 'Cancel', command: { type: 'uuid', id: null }, tone: 'danger' }]);
    expect(required.secondaryButtons).toEqual([]);

    const chosen = deriveInteraction(view(), parsePrompt('GAME_TARGET', {
      message: 'Choose up to two targets', targets: ['bear', OPPONENT], flag: false, options: { chosenTargets: ['bear'] },
    }));
    expect(chosen.mainButton).toMatchObject({ label: 'Done', command: { type: 'uuid', id: null } });
  });

  test('targets among cards that are not on the board use a card picker', () => {
    const interaction = deriveInteraction(view(), parsePrompt('GAME_TARGET', {
      message: 'Search your library', cardsView1: { c1: { name: 'Forest' } }, flag: true,
    }));
    expect(interaction.mode).toBe('pickCards');
    expect(interaction.clickable.has('c1')).toBe(true);
  });

  test('an optional pick among cards can finish with no card, e.g. a search that finds nothing', () => {
    const interaction = deriveInteraction(view(), parsePrompt('GAME_TARGET', {
      message: 'Search your library for up to one basic land', cardsView1: { c1: { name: 'Shock' } }, flag: false,
    }));
    expect(interaction.mainButton).toMatchObject({ label: 'Done', command: { type: 'uuid', id: null } });
    expect(interaction.secondaryButtons).toEqual([]);
  });

  test('mana payment never offers a boolean that would cancel by accident', () => {
    const interaction = deriveInteraction(view(), parsePrompt('GAME_PLAY_MANA', { message: 'Pay {R}' }));
    expect(interaction.mode).toBe('payMana');
    expect(interaction.mainButton).toBeNull();
    expect(interaction.secondaryButtons).toEqual([
      { label: 'Cancel', command: { type: 'boolean', value: false }, tone: 'danger' },
    ]);
  });

  test('yes/no questions have no keyboard default', () => {
    const interaction = deriveInteraction(view(), parsePrompt('GAME_ASK', { message: 'Pay {1}?' }));
    expect(interaction.mode).toBe('question');
    expect(interaction.mainButton?.shortcut).toBeUndefined();
    expect(interaction.secondaryButtons[0].shortcut).toBeUndefined();
  });

  test('no prompt means waiting', () => {
    expect(deriveInteraction(view({ priorityPlayerName: 'Opponent' }), null)).toMatchObject({
      mode: 'waiting', headline: 'Waiting for Opponent', mainButton: null,
    });
  });
});

describe('structural sharing', () => {
  test('unchanged parts keep their identity', () => {
    const first = view();
    const second = structuralShare(first, view({ turn: 4 }));
    expect(second).not.toBe(first);
    expect(second.players).toBe(first.players);
    expect(second.canPlayObjects).toBe(first.canPlayObjects);
    expect(second.turn).toBe(4);
    expect(structuralShare(first, view())).toBe(first);
  });
});

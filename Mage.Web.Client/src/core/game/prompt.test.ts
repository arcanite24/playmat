import { describe, expect, test } from 'vitest';
import type { AbilityPickerView, CardView, GameClientMessage, GameView } from '../../protocol/generated/views';
import { abilityChooserSource, isPromptEvent, parsePrompt, promptGameView, PROMPT_EVENTS, stripMarkup } from './prompt';

// Fixture payloads shaped like the server's dialog callbacks (GameClientMessage / AbilityPickerView)
const GAME_VIEW = { turn: 3 } as unknown as GameView;
const card = (id: string, name: string) => ({ id, name } as unknown as CardView);

const fixtures = {
  priority: {
    message: 'Play spells and abilities.',
    gameView: GAME_VIEW,
  } satisfies GameClientMessage,
  attackers: {
    message: 'Select attackers',
    options: { possibleAttackers: ['a1', 'a2', 7], specialButton: 'All attack' },
  } as unknown as GameClientMessage,
  blockers: {
    message: 'Select blockers',
    options: { possibleBlockers: ['b1'] },
  } as unknown as GameClientMessage,
  target: {
    message: 'Select a target <font color=\'#FFFF00\'>creature</font>',
    targets: ['t1', 't2'],
    flag: true,
    options: { chosenTargets: ['t0'], 'UI.right.btn.text': 'Done' },
  } as unknown as GameClientMessage,
  targetFromCards: {
    message: 'Search your library',
    cardsView1: { c1: card('c1', 'Forest'), c2: card('c2', 'Island') },
    flag: false,
  } satisfies GameClientMessage,
  targetFromOptions: {
    message: 'Choose',
    options: { possibleTargets: ['p1'] },
  } as unknown as GameClientMessage,
  mulligan: {
    message: 'Mulligan down to 6 cards?',
    options: { 'UI.left.btn.text': 'Mulligan', 'UI.right.btn.text': 'Keep' },
  } as unknown as GameClientMessage,
  ask: { message: 'Use the ability?' } satisfies GameClientMessage,
  ability: {
    message: 'Choose ability',
    choices: { 'ab-1': '{T}: Add {G}.', 'ab-2': 'Cancel' },
    gameView: GAME_VIEW,
  } satisfies AbilityPickerView,
  pile: {
    message: 'Choose a pile',
    cardsView1: { x: card('x', 'Bolt') },
    cardsView2: { y: card('y', 'Shock'), z: card('z', 'Opt') },
  } satisfies GameClientMessage,
  choice: {
    message: 'Choose a color',
    choice: { message: 'Choose color', choices: ['Red', 'Green'], required: true },
  } as unknown as GameClientMessage,
  mana: {
    message: 'Pay {2}{R}',
    options: { specialButton: 'Convoke' },
  } as unknown as GameClientMessage,
  amount: { message: 'How many?', min: 1, max: 5 } satisfies GameClientMessage,
  multiAmount: {
    message: 'Distribute damage',
    messages: [{ message: 'Goblin', min: 0, max: 3, defaultValue: 1 }],
    min: 3,
    max: 3,
    options: { title: 'Distribute', header: 'Split 3 damage', canCancel: true },
  } as unknown as GameClientMessage,
};

describe('parsePrompt', () => {
  test('GAME_SELECT without options is the priority prompt', () => {
    const prompt = parsePrompt('GAME_SELECT', fixtures.priority);
    expect(prompt).toMatchObject({ kind: 'priority', text: 'Play spells and abilities.', options: {} });
  });

  test('GAME_SELECT with possible attackers declares attackers and keeps only ids', () => {
    expect(parsePrompt('GAME_SELECT', fixtures.attackers)).toMatchObject({
      kind: 'declareAttackers',
      possibleAttackers: ['a1', 'a2'],
      allAttackLabel: 'All attack',
    });
  });

  test('GAME_SELECT with possible blockers declares blockers', () => {
    expect(parsePrompt('GAME_SELECT', fixtures.blockers)).toMatchObject({
      kind: 'declareBlockers',
      possibleBlockers: ['b1'],
    });
  });

  test('GAME_TARGET reads targets, required flag, chosen targets and the done label', () => {
    const prompt = parsePrompt('GAME_TARGET', fixtures.target);
    expect(prompt).toMatchObject({
      kind: 'target',
      text: 'Select a target creature',
      targets: ['t1', 't2'],
      cards: null,
      required: true,
      chosen: ['t0'],
      doneLabel: 'Done',
    });
  });

  test('GAME_TARGET falls back to off-board cards, then to possibleTargets', () => {
    const fromCards = parsePrompt('GAME_TARGET', fixtures.targetFromCards);
    expect(fromCards).toMatchObject({ kind: 'target', targets: ['c1', 'c2'], required: false, doneLabel: null });
    expect(fromCards.kind === 'target' && fromCards.cards && Object.keys(fromCards.cards)).toEqual(['c1', 'c2']);
    expect(parsePrompt('GAME_TARGET', fixtures.targetFromOptions)).toMatchObject({ targets: ['p1'], cards: null });
  });

  test('GAME_ASK detects the mulligan question by its button labels', () => {
    expect(parsePrompt('GAME_ASK', fixtures.mulligan)).toMatchObject({
      kind: 'ask',
      yesLabel: 'Mulligan',
      noLabel: 'Keep',
      isMulligan: true,
    });
    expect(parsePrompt('GAME_ASK', fixtures.ask)).toMatchObject({
      kind: 'ask',
      yesLabel: 'Yes',
      noLabel: 'No',
      isMulligan: false,
    });
  });

  test('GAME_CHOOSE_ABILITY keeps the server order of choices', () => {
    expect(parsePrompt('GAME_CHOOSE_ABILITY', fixtures.ability)).toMatchObject({
      kind: 'chooseAbility',
      choices: [{ id: 'ab-1', text: '{T}: Add {G}.' }, { id: 'ab-2', text: 'Cancel' }],
    });
  });

  test('GAME_CHOOSE_PILE splits both piles', () => {
    const prompt = parsePrompt('GAME_CHOOSE_PILE', fixtures.pile);
    expect(prompt.kind).toBe('choosePile');
    if (prompt.kind !== 'choosePile') return;
    expect(prompt.pile1.map((c) => c.name)).toEqual(['Bolt']);
    expect(prompt.pile2.map((c) => c.name)).toEqual(['Shock', 'Opt']);
  });

  test('GAME_CHOOSE_CHOICE carries the choice', () => {
    expect(parsePrompt('GAME_CHOOSE_CHOICE', fixtures.choice)).toMatchObject({
      kind: 'chooseChoice',
      choice: { choices: ['Red', 'Green'], required: true },
    });
  });

  test('GAME_PLAY_MANA and GAME_PLAY_XMANA are both mana payments', () => {
    expect(parsePrompt('GAME_PLAY_MANA', fixtures.mana)).toMatchObject({ kind: 'playMana', specialLabel: 'Convoke', isX: false });
    expect(parsePrompt('GAME_PLAY_XMANA', { message: 'X' })).toMatchObject({ kind: 'playMana', specialLabel: null, isX: true });
    // the server flags a special payment on the view only: the button is named after the spell's keyword
    const delve = { message: 'Pay {1}{B}', gameView: { special: true, stack: { s: { id: 's', name: 'Murderous Cut', rules: ['Delve <i>(Each card you exile from your graveyard while casting this spell pays for {1}.)</i>', 'Destroy target creature.'] } } } };
    expect(parsePrompt('GAME_PLAY_MANA', delve as never)).toMatchObject({ specialLabel: 'Delve' });
    expect(parsePrompt('GAME_PLAY_MANA', { message: 'Pay {1}', gameView: { special: true, stack: {} } } as never)).toMatchObject({ specialLabel: 'Special payment' });
    expect(parsePrompt('GAME_PLAY_MANA', { message: 'Pay {1}', gameView: { special: false } } as never)).toMatchObject({ specialLabel: null });
  });

  test('GAME_GET_AMOUNT reads the bounds', () => {
    expect(parsePrompt('GAME_GET_AMOUNT', fixtures.amount)).toMatchObject({ kind: 'amount', min: 1, max: 5 });
    expect(parsePrompt('GAME_GET_AMOUNT', {})).toMatchObject({ kind: 'amount', min: 0, max: 0, text: '' });
  });

  test('GAME_GET_MULTI_AMOUNT reads items, bounds and labels', () => {
    expect(parsePrompt('GAME_GET_MULTI_AMOUNT', fixtures.multiAmount)).toMatchObject({
      kind: 'multiAmount',
      items: [{ message: 'Goblin', min: 0, max: 3, defaultValue: 1 }],
      min: 3,
      max: 3,
      title: 'Distribute',
      header: 'Split 3 damage',
      canCancel: true,
    });
  });

  test('rejects non-prompt events', () => {
    expect(() => parsePrompt('GAME_UPDATE' as never, {} as never)).toThrow(/Not a prompt event/);
  });
});

describe('prompt helpers', () => {
  test('every prompt event is recognised and nothing else is', () => {
    for (const event of PROMPT_EVENTS) expect(isPromptEvent(event)).toBe(true);
    expect(isPromptEvent('GAME_UPDATE')).toBe(false);
    expect(isPromptEvent('CHATMESSAGE')).toBe(false);
  });

  test('promptGameView reads the view from either payload shape', () => {
    expect(promptGameView('GAME_SELECT', fixtures.priority)).toBe(GAME_VIEW);
    expect(promptGameView('GAME_CHOOSE_ABILITY', fixtures.ability)).toBe(GAME_VIEW);
    expect(promptGameView('GAME_ASK', null)).toBeUndefined();
  });

  test('stripMarkup removes tags and decodes entities', () => {
    expect(stripMarkup('Pay <b>2</b>&nbsp;life&mdash;or&hellip;<br/>not &amp; &#65;')).toBe('Pay 2 life—or… not & A');
    expect(stripMarkup(undefined)).toBe('');
  });
});

describe('abilityChooserSource', () => {
  test('names what the ability chooser is for', () => {
    expect(abilityChooserSource(stripMarkup("Choose spell or ability to play<br><font color='#ccc'>Evolving Wilds [a16]</font>"))).toBe('Evolving Wilds');
    expect(abilityChooserSource('Choose spell or ability to play for FREE Fire // Ice [0b1]')).toBe('Fire // Ice');
    expect(abilityChooserSource('Choose mode')).toBeNull();
  });
});

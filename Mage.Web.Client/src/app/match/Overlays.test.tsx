import userEvent from '@testing-library/user-event';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import type { Command, Interaction } from '../../core/game/interaction';
import { parsePrompt } from '../../core/game/prompt';
import type { CardView, GameClientMessage } from '../../protocol/generated/views';
import { CardPicker, ChoicePanel, GameOverOverlay, MulliganOverlay, StartingPlayerOverlay, ZoneViewer } from './Overlays';
import { useMatchUi } from './matchUi';

const card = (id: string, name: string) => ({ id, name, expansionSetCode: 'M10', cardNumber: '1' } as CardView);

function interaction(partial: Partial<Interaction>): Interaction {
  return {
    mode: 'panel',
    prompt: null,
    headline: '',
    clickable: new Map(),
    selected: new Set(),
    mainButton: null,
    secondaryButtons: [],
    ...partial,
  };
}

function answers() {
  const sent: Command[] = [];
  const onCommand = vi.fn((command: Command) => {
    sent.push(command);
  });
  return { sent, onCommand };
}

const dialog = (name: string | RegExp) => screen.getByRole('dialog', { name });
const button = (name: string | RegExp) => screen.getByRole('button', { name });

describe('MulliganOverlay', () => {
  const prompt = parsePrompt('GAME_ASK', {
    message: 'Mulligan down to 6 cards?',
    options: { 'UI.left.btn.text': 'Mulligan', 'UI.right.btn.text': 'Keep' },
  } as unknown as GameClientMessage);

  test('shows the opening hand and answers keep or mulligan', () => {
    const { sent, onCommand } = answers();
    render(
      <MulliganOverlay
        hand={[card('a', 'Forest'), card('b', 'Llanowar Elves')]}
        interaction={interaction({ mode: 'mulligan', prompt })}
        sleeve="default"
        onCommand={onCommand}
      />,
    );
    expect(dialog('Opening hand')).toBeTruthy();
    expect(screen.getByText('Mulligan down to 6 cards?')).toBeTruthy();
    fireEvent.click(button('Keep'));
    fireEvent.click(button('Mulligan'));
    expect(sent).toEqual([{ type: 'boolean', value: false }, { type: 'boolean', value: true }]);
  });

  test('renders nothing without an ask prompt', () => {
    const { container } = render(
      <MulliganOverlay hand={[]} interaction={interaction({ prompt: null })} sleeve="default" onCommand={vi.fn()} />,
    );
    expect(container.innerHTML).toBe('');
  });
});

describe('CardPicker', () => {
  test('toggles clickable cards, ignores the others and finishes with the main button', () => {
    const { sent, onCommand } = answers();
    const done: Command = { type: 'boolean', value: true };
    render(
      <CardPicker
        title="Search your library"
        cards={[card('f', 'Forest'), card('i', 'Island')]}
        interaction={interaction({
          mode: 'pickCards',
          headline: 'Choose a basic land',
          clickable: new Map([['f', { type: 'uuid', id: 'f' }]]),
          selected: new Set(['f']),
          mainButton: { label: 'Done', command: done, tone: 'primary' },
        })}
        sleeve="default"
        onCommand={onCommand}
      />,
    );
    const picker = dialog('Search your library');
    const forest = within(picker).getByRole('button', { name: 'Forest, chosen' });
    expect(forest.getAttribute('aria-pressed')).toBe('true');
    // only what can be chosen, until Show all
    expect(within(picker).queryByRole('button', { name: 'Island' })).toBeNull();
    fireEvent.click(within(picker).getByRole('button', { name: 'Show all (2)' }));
    const island = within(picker).getByRole('button', { name: 'Island' }) as HTMLButtonElement;
    expect(island.disabled).toBe(true);

    fireEvent.click(forest);
    fireEvent.click(island);
    fireEvent.click(button('Done'));
    expect(sent).toEqual([{ type: 'uuid', id: 'f' }, done]);
  });

  test('can be hidden to look at the board and brought back', () => {
    render(
      <CardPicker title="Choose" cards={[card('f', 'Forest')]} interaction={interaction({})} sleeve="default" onCommand={vi.fn()} />,
    );
    fireEvent.click(button('See the board'));
    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent.click(button('Back to choice'));
    expect(dialog('Choose')).toBeTruthy();
  });
});

describe('ChoicePanel', () => {
  test('chooseAbility: picks by click or number key, cancels with Escape', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_CHOOSE_ABILITY', {
      message: 'Choose mode',
      choices: { m1: '1. Deal 2 damage', m2: '2. Draw a card' },
    });
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect(dialog('Choose mode')).toBeTruthy();
    fireEvent.click(button(/Draw a card/));
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: 'Escape' });
    fireEvent.click(button('Cancel'));
    expect(sent).toEqual([
      { type: 'uuid', id: 'm2' },
      { type: 'uuid', id: 'm1' },
      { type: 'uuid', id: null },
      { type: 'uuid', id: null },
    ]);
  });

  test('mana abilities use symbols, retain complex costs, and answer with the original id', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_CHOOSE_ABILITY', {
      message: 'Choose ability',
      choices: { free: '{T}: Add {C}.', pain: '{T}: Add {U}. Reef deals 1 damage to you.' },
    });
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect(dialog('Choose mana')).toBeTruthy();
    const free = button('{T}: Add {C}.');
    const pain = button('{T}: Add {U}. Reef deals 1 damage to you.');
    expect(free.querySelector('.ms-c')).toBeTruthy();
    expect(pain.querySelector('.ms-u')).toBeTruthy();
    expect(pain.textContent).toContain('Reef deals 1 damage to you.');
    // like every choice, the panel takes focus so a held Space or Enter can't pick the first mana by accident
    expect(document.activeElement).toBe(dialog('Choose mana'));
    await user.keyboard('{ArrowRight}{ArrowRight}{Enter}');
    await user.keyboard('1');
    await user.keyboard('{Escape}');
    expect(sent).toEqual([{ type: 'uuid', id: 'pain' }, { type: 'uuid', id: 'free' }, { type: 'uuid', id: null }]);
    expect(document.activeElement).toBe(button('Cancel'));
    await user.tab();
    expect(document.activeElement).toBe(free);
  });

  test('a mixed mana and non-mana list retains the ordinary ability picker', () => {
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_ABILITY', {
      message: 'Choose ability', choices: { mana: '{T}: Add {G}.', draw: '{T}: Draw a card.' },
    })} onCommand={vi.fn()} />);
    expect(dialog('Choose ability')).toBeTruthy();
    expect(screen.queryByRole('dialog', { name: 'Choose mana' })).toBeNull();
    expect(button(/Draw a card/)).toBeTruthy();
  });

  test.each([
    ['Treasure', '{T}, Sacrifice this artifact: Add one mana of any color.'],
    ['Lotus Petal', 'Sacrifice Lotus Petal: Add one mana of any color.'],
    ['Jeweled Lotus', '{T}, Sacrifice Jeweled Lotus: Add three mana of any one color.'],
    ['Crystal Vein', '{T}, Sacrifice Crystal Vein: Add {C}{C}.'],
  ])('a lone mana ability confirms using %s before activating it', (source, text) => {
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_ABILITY', {
      message: `Choose spell or ability to play<br><font color='#ccc'>${source} [a16]</font>`,
      choices: { sacrifice: text },
    })} onCommand={onCommand} />);
    const panel = dialog(`Use ${source}?`);
    expect(screen.queryByRole('dialog', { name: 'Choose mana' })).toBeNull();
    expect(within(panel).getByText(/Sacrifice/)).toBeTruthy();
    expect(panel.querySelector('.ms-w, .ms-u, .ms-b, .ms-r, .ms-g')).toBeNull();
    expect(sent).toEqual([]);
    fireEvent.click(button(/Sacrifice/));
    fireEvent.click(button('Cancel'));
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(sent).toEqual([
      { type: 'uuid', id: 'sacrifice' }, { type: 'uuid', id: null },
      { type: 'uuid', id: 'sacrifice' }, { type: 'uuid', id: null },
    ]);
  });

  test.each([
    'one mana of any color',
    'two mana of any one color',
    'three mana in any combination of colors',
  ])('any-color output stays a single labelled ability in a mana list: %s', (output) => {
    const { sent, onCommand } = answers();
    const text = `{T}, Sacrifice this artifact: Add ${output}.`;
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_ABILITY', {
      message: 'Choose ability', choices: { colorless: '{T}: Add {C}.', any: text },
    })} onCommand={onCommand} />);
    expect(dialog('Choose mana')).toBeTruthy();
    const any = button(text);
    expect(within(any).getByText(/Sacrifice this artifact: Add/)).toBeTruthy();
    expect(any.querySelector('.ms-w, .ms-u, .ms-b, .ms-r, .ms-g')).toBeNull();
    expect(screen.getAllByRole('button')).toHaveLength(3);
    fireEvent.click(any);
    expect(sent).toEqual([{ type: 'uuid', id: 'any' }]);
  });

  test('required colors preserve sort order and string values; shortcuts ignore modifiers and details', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_CHOICE', {
      choice: { message: 'Choose color', choices: ['Green', 'Blue'], sortData: { Blue: 0, Green: 1 }, required: true, manaColorChoice: true },
    })} onCommand={onCommand} />);
    expect(button('Blue').querySelector('.ms-u')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Skip' })).toBeNull();
    fireEvent.keyDown(window, { key: '1', ctrlKey: true });
    fireEvent.keyDown(window, { key: 'Escape' });
    useMatchUi.setState({ detail: { id: 'source', card: card('source', 'Birds of Paradise') } });
    fireEvent.keyDown(window, { key: '1' });
    useMatchUi.setState({ detail: null });
    expect(sent).toEqual([]);
    await user.keyboard('2');
    expect(sent).toEqual([{ type: 'string', value: 'Green' }]);
  });

  test('chooseAbility: a lone ability (the server confirms a sacrifice) reads as a plain question', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_CHOOSE_ABILITY', {
      message: "Choose spell or ability to play<br><font color='#ccc'>Evolving Wilds [a16]</font>",
      choices: { a: '{T}, Sacrifice Evolving Wilds: Search your library for a basic land card.' },
    });
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect(screen.getByRole('heading', { name: 'Use Evolving Wilds?' })).toBeTruthy();
    fireEvent.click(button('Cancel'));
    expect(sent).toEqual([{ type: 'uuid', id: null }]);
  });

  test('amount: announcing X shows the range up to the mana the player has', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_GET_AMOUNT', { message: 'Announce the value for {X} (source: Fireball [2ec])', min: 0, max: 2147483647 });
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} xLimit={3} />);
    expect(screen.getByRole('heading', { name: /Announce the value for X \(source: Fireball\)/ })).toBeTruthy();
    expect(screen.getByText('0 to 3')).toBeTruthy();
    expect(screen.getByText('You have 3 mana to pay with.')).toBeTruthy();
    fireEvent.change(screen.getByRole('slider'), { target: { value: '2' } });
    fireEvent.click(button('Choose 2'));
    expect(sent).toEqual([{ type: 'integer', value: 2 }]);
  });

  test('chooseChoice: answers with the key of a keyed choice, and Skip when optional', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_CHOOSE_CHOICE', {
      message: 'Choose a color',
      choice: { message: 'Choose a color', keyChoices: { R: 'Red', G: 'Green' }, required: false },
    } as unknown as GameClientMessage);
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    fireEvent.click(button(/Green/));
    fireEvent.click(button('Skip'));
    expect(sent).toEqual([{ type: 'string', value: 'G' }, { type: 'string', value: '' }]);
  });

  test('chooseChoice: long lists can be searched', () => {
    const { sent, onCommand } = answers();
    const names = ['Angel', 'Beast', 'Cat', 'Dragon', 'Elf', 'Faerie', 'Goblin', 'Human', 'Insect', 'Jackal'];
    const prompt = parsePrompt('GAME_CHOOSE_CHOICE', {
      message: 'Choose a creature type',
      choice: { message: 'Choose a creature type', choices: names, required: true },
    } as unknown as GameClientMessage);
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect(screen.queryByRole('button', { name: 'Skip' })).toBeNull();
    fireEvent.change(screen.getByRole('textbox', { name: 'Search choices' }), { target: { value: 'gob' } });
    const options = within(dialog('Choose a creature type')).getAllByRole('button');
    expect(options).toHaveLength(1);
    fireEvent.click(options[0]);
    expect(sent).toEqual([{ type: 'string', value: 'Goblin' }]);
  });

  test('choosePile: answers true for the first pile and false for the second', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_CHOOSE_PILE', {
      message: 'Choose a pile',
      cardsView1: { a: card('a', 'Bolt') },
      cardsView2: { b: card('b', 'Shock'), c: card('c', 'Opt') },
    });
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    fireEvent.click(button(/Pile 2 · 2 cards/));
    fireEvent.click(button(/Pile 1 · 1 card/));
    expect(sent).toEqual([{ type: 'boolean', value: false }, { type: 'boolean', value: true }]);
  });

  test('amount: steps within the bounds and sends the number', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_GET_AMOUNT', { message: 'Choose X', min: 1, max: 3 });
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect((button('Less') as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(button('More'));
    fireEvent.click(button('More'));
    fireEvent.click(button('More'));
    expect((button('More') as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(button('Choose 3'));
    expect(sent).toEqual([{ type: 'integer', value: 3 }]);
  });

  test('multiAmount: only a valid split can be sent', () => {
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_GET_MULTI_AMOUNT', {
      message: 'Divide 3 damage',
      messages: [{ message: 'Goblin', min: 0, max: 3, defaultValue: 0 }, { message: 'Elf', min: 0, max: 3, defaultValue: 0 }],
      min: 3,
      max: 3,
      options: { canCancel: true },
    } as unknown as GameClientMessage);
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect((button('Done') as HTMLButtonElement).disabled).toBe(true);
    const [goblin, elf] = screen.getAllByRole('spinbutton');
    fireEvent.change(goblin, { target: { value: '2' } });
    fireEvent.change(elf, { target: { value: '1' } });
    fireEvent.click(button('Done'));
    fireEvent.click(button('Cancel'));
    expect(sent).toEqual([{ type: 'string', value: '2 1' }, { type: 'string', value: '' }]);
  });

  test('prompts that live on the board render no panel', () => {
    const { container } = render(<ChoicePanel prompt={parsePrompt('GAME_SELECT', { message: 'Play' })} onCommand={vi.fn()} />);
    expect(container.innerHTML).toBe('');
  });
});

describe('StartingPlayerOverlay', () => {
  test('chooses who starts', () => {
    const onChoose = vi.fn();
    render(<StartingPlayerOverlay me={{ id: 'me', name: 'Me' }} opponents={[{ id: 'ai', name: 'Computer' }]} onChoose={onChoose} />);
    expect(dialog('You choose who starts')).toBeTruthy();
    fireEvent.click(button('Draw first'));
    fireEvent.click(button('Play first'));
    expect(onChoose.mock.calls).toEqual([['ai'], ['me']]);
  });
});

describe('GameOverOverlay', () => {
  test('shows the result and the match score, and leaves or plays again', () => {
    const onLeave = vi.fn();
    const onPlayAgain = vi.fn();
    render(
      <GameOverOverlay
        message="You won the game"
        endInfo={{ won: true, wins: 1, loses: 0, winsNeeded: 2, gameInfo: 'Computer conceded.' }}
        myName="me"
        onLeave={onLeave}
        onPlayAgain={onPlayAgain}
      />,
    );
    expect(screen.getByRole('heading', { name: 'Victory' })).toBeTruthy();
    expect(screen.getByText('You won the game.')).toBeTruthy();
    fireEvent.click(button('Back to Play'));
    fireEvent.click(button('Play again'));
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(onPlayAgain).toHaveBeenCalledTimes(1);
  });

  test('a loss without end info reads from the message', () => {
    render(<GameOverOverlay message="You lost the game" endInfo={null} myName="me" onLeave={vi.fn()} />);
    expect(screen.getByRole('heading', { name: 'Defeat' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Play again' })).toBeNull();
  });

  test("says the match result and score in plain words, not the server's line", () => {
    const players = [{ playerId: 'me', wins: 2 }, { playerId: 'bob', wins: 0 }];
    render(
      <GameOverOverlay
        message="Player bashalice is the winner"
        endInfo={{ won: true, wins: 2, winsNeeded: 2, clientPlayer: { playerId: 'me' }, players, matchInfo: 'You won the match!' }}
        myName="bashalice"
        onLeave={vi.fn()}
        leaveLabel="Back to Tables"
      />,
    );
    expect(screen.getByRole('heading', { name: 'Victory' })).toBeTruthy();
    expect(screen.getByText('You won the match 2–0.')).toBeTruthy();
    expect(screen.queryByText(/is the winner/)).toBeNull();
    expect(button('Back to Tables')).toBeTruthy();
  });

  test('tells who won from the server line when no end-of-game info came', () => {
    render(<GameOverOverlay message="Player bashalice is the winner" endInfo={null} myName="bashbob" onLeave={vi.fn()} />);
    expect(screen.getByRole('heading', { name: 'Defeat' })).toBeTruthy();
    expect(screen.getByText('You lost the game.')).toBeTruthy();
  });
});

describe('ZoneViewer', () => {
  test('lists the cards and closes with the button, Escape or the scrim', () => {
    const onClose = vi.fn();
    render(<ZoneViewer title="Your graveyard" cards={[card('a', 'Bolt')]} onClose={onClose} />);
    const viewer = dialog('Your graveyard');
    expect(viewer.querySelector('[data-card-id="a"]')).toBeTruthy();
    fireEvent.click(button('Close'));
    fireEvent.keyDown(window, { key: 'Escape' });
    fireEvent.click(viewer);
    expect(onClose).toHaveBeenCalledTimes(3);
  });

  test('says when the zone is empty', () => {
    render(<ZoneViewer title="Exile" cards={[]} onClose={vi.fn()} />);
    expect(screen.getByText('Empty.')).toBeTruthy();
  });

  test('owns focus until closed and leaves a dialog above it in control', () => {
    const onClose = vi.fn();
    render(<button>Graveyard</button>);
    button('Graveyard').focus();
    const viewer = render(<ZoneViewer title="Your graveyard" cards={[]} onClose={onClose} />);
    expect(document.activeElement).toBe(dialog('Your graveyard'));
    const layer = render(<div role="dialog"><textarea aria-label="Chat" /></div>);
    fireEvent.keyDown(window, { key: 'Escape' });
    fireEvent.keyDown(screen.getByRole('textbox'), { key: 'Escape' });
    expect(onClose).not.toHaveBeenCalled();
    layer.unmount();
    viewer.unmount();
    expect(document.activeElement).toBe(button('Graveyard'));
  });
});

describe('choice keyboard ownership', () => {
  const abilityPrompt = () => parsePrompt('GAME_CHOOSE_ABILITY', {
    message: 'Choose mode', choices: { a: 'Draw a card', b: 'Gain life' },
  });

  test.each([
    abilityPrompt(),
    parsePrompt('GAME_CHOOSE_CHOICE', { choice: { choices: ['Red', 'Green'], required: true } } as unknown as GameClientMessage),
    parsePrompt('GAME_GET_AMOUNT', { message: 'Choose X', min: 0, max: 3 }),
  ])('Space and Enter wait for navigation when a $kind panel opens', async (prompt) => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={prompt} onCommand={onCommand} />);
    expect(document.activeElement).toBe(screen.getByRole('dialog'));
    await user.keyboard(' {Enter} {Enter}');
    (document.activeElement as HTMLElement).blur();
    expect(document.activeElement).toBe(document.body);
    await user.keyboard(' {Enter}');
    expect(sent).toEqual([]);
  });

  test('opening hand and starting-player dialogs wait for deliberate focus', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    const prompt = parsePrompt('GAME_ASK', {
      message: 'Mulligan?', options: { 'UI.left.btn.text': 'Mulligan', 'UI.right.btn.text': 'Keep' },
    } as unknown as GameClientMessage);
    const opening = render(<MulliganOverlay hand={[]} interaction={interaction({ mode: 'mulligan', prompt })} sleeve="default" onCommand={onCommand} />);
    expect(document.activeElement).toBe(dialog('Opening hand'));
    await user.keyboard(' {Enter}');
    expect(sent).toEqual([]);
    opening.unmount();
    const onChoose = vi.fn();
    render(<StartingPlayerOverlay me={{ id: 'me', name: 'Me' }} opponents={[{ id: 'ai', name: 'Computer' }]} onChoose={onChoose} />);
    expect(document.activeElement).toBe(dialog('You choose who starts'));
    await user.keyboard(' {Enter}');
    expect(onChoose).not.toHaveBeenCalled();
    await user.tab();
    await user.keyboard('{Enter}');
    expect(onChoose).toHaveBeenCalledWith('ai');
  });

  test('Enter confirms the arrow-focused choice once', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={abilityPrompt()} onCommand={onCommand} />);
    await user.keyboard('{ArrowDown}{Enter}');
    expect(sent).toEqual([{ type: 'uuid', id: 'a' }]);
  });

  test('arrows move focus, Tab stays inside and Escape cancels', () => {
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={abilityPrompt()} onCommand={onCommand} />);
    expect(document.activeElement).toBe(dialog('Choose mode'));
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowUp' });
    expect(document.activeElement).toBe(button(/Gain life/));
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(button(/Draw a card/));
    fireEvent.keyDown(document.activeElement!, { key: 'End' });
    expect(document.activeElement).toBe(button(/Gain life/));
    fireEvent.keyDown(document.activeElement!, { key: 'Home' });
    expect(document.activeElement).toBe(button(/Draw a card/));
    fireEvent.keyDown(document.activeElement!, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(button('Cancel'));
    fireEvent.keyDown(document.activeElement!, { key: 'Tab' });
    expect(document.activeElement).toBe(button(/Draw a card/));
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(sent).toEqual([{ type: 'uuid', id: null }]);
  });

  test('ignores typing, modifiers and repeats, and only the top dialog answers', () => {
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={abilityPrompt()} onCommand={onCommand} />);
    fireEvent.keyDown(window, { key: '1', ctrlKey: true });
    fireEvent.keyDown(window, { key: '1', repeat: true });
    const layer = render(<div role="dialog"><textarea aria-label="Chat" /></div>);
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(screen.getByRole('textbox'), { key: 'Escape' });
    layer.unmount();
    expect(sent).toEqual([]);
    fireEvent.keyDown(window, { key: '1' });
    expect(sent).toEqual([{ type: 'uuid', id: 'a' }]);
  });

  test('0 chooses the tenth option and filtered lists use their visible order', () => {
    const { sent, onCommand } = answers();
    const choices = Array.from({ length: 12 }, (_, index) => `Choice ${index + 1}`);
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_CHOICE', { choice: { choices, required: true } } as unknown as GameClientMessage)} onCommand={onCommand} />);
    fireEvent.keyDown(window, { key: '0' });
    expect(sent).toEqual([{ type: 'string', value: 'Choice 10' }]);
    const search = screen.getByRole('textbox');
    fireEvent.keyDown(search, { key: '1' });
    expect(sent).toHaveLength(1);
    fireEvent.change(search, { target: { value: 'Choice 12' } });
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(sent).toEqual([{ type: 'string', value: 'Choice 10' }, { type: 'string', value: 'Choice 12' }]);
  });

  test('optional choices can be skipped with Escape or Alt+Enter', () => {
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_CHOICE', { choice: { choices: ['Red'], required: false } } as unknown as GameClientMessage)} onCommand={onCommand} />);
    fireEvent.keyDown(window, { key: 'Escape' });
    fireEvent.keyDown(window, { key: 'Enter', altKey: true });
    expect(sent).toEqual([{ type: 'string', value: '' }, { type: 'string', value: '' }]);
  });

  test('Alt+Enter cannot select a focused choice when no secondary option exists', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_CHOICE', { choice: { choices: ['Red'], required: true } } as unknown as GameClientMessage)} onCommand={onCommand} />);
    await user.keyboard('{ArrowDown}');
    await user.keyboard('{Alt>}{Enter}{/Alt}');
    expect(sent).toEqual([]);
    await user.keyboard('{Enter}');
    expect(sent).toEqual([{ type: 'string', value: 'Red' }]);
  });

  test('piles support number shortcuts', () => {
    const { sent, onCommand } = answers();
    render(<ChoicePanel prompt={parsePrompt('GAME_CHOOSE_PILE', { cardsView1: {}, cardsView2: {} })} onCommand={onCommand} />);
    fireEvent.keyDown(window, { key: '2' });
    expect(sent).toEqual([{ type: 'boolean', value: false }]);
  });

  test('card pickers select eligible cards by number and finish only with deliberate focus', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    render(<CardPicker title="Choose cards" cards={[card('f', 'Forest'), card('i', 'Island')]} interaction={interaction({ clickable: new Map([['i', { type: 'uuid', id: 'i' }]]), mainButton: { label: 'Done', command: { type: 'boolean', value: true }, tone: 'primary' } })} sleeve="default" onCommand={onCommand} />);
    expect(document.activeElement).toBe(dialog('Choose cards'));
    await user.keyboard(' {Enter}');
    expect(sent).toEqual([]);
    fireEvent.keyDown(window, { key: '1' });
    expect(document.activeElement).toBe(button('Island'));
    await user.tab();
    expect(document.activeElement).toBe(button('Done'));
    await user.keyboard('{Enter}');
    expect(sent).toEqual([{ type: 'uuid', id: 'i' }, { type: 'boolean', value: true }]);
  });

  test('Space mashing cannot finish a library search with no selectable cards', async () => {
    const user = userEvent.setup();
    const { sent, onCommand } = answers();
    render(<CardPicker title="Search your library" cards={[]} interaction={interaction({ mainButton: { label: 'Done', command: { type: 'uuid', id: null }, tone: 'primary' } })} sleeve="default" onCommand={onCommand} />);
    expect(document.activeElement).toBe(dialog('Search your library'));
    await user.keyboard(' {Enter}   ');
    expect(sent).toEqual([]);
    await user.tab();
    await user.tab();
    expect(document.activeElement).toBe(button('Done'));
    await user.keyboard('{Enter}');
    expect(sent).toEqual([{ type: 'uuid', id: null }]);
  });
});

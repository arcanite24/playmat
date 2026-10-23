import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import userEvent from '@testing-library/user-event';
import type { Command, Interaction } from '../../core/game/interaction';
import { ActionCluster } from './ActionCluster';

function interaction(partial: Partial<Interaction>): Interaction {
  return {
    mode: 'priority',
    prompt: null,
    headline: '',
    clickable: new Map(),
    selected: new Set(),
    mainButton: null,
    secondaryButtons: [],
    ...partial,
  };
}

const CANCEL = { label: 'Cancel', command: { type: 'boolean', value: false } as Command, tone: 'danger' as const };

function cluster(props: { interaction: Interaction; undoable?: boolean }) {
  const sent: Command[] = [];
  render(
    <ActionCluster
      awaiting={false}
      status={null}
      canAct
      special={false}
      holdingPriority={false}
      onCommand={(command) => sent.push(command)}
      {...props}
    />,
  );
  return sent;
}

describe('ActionCluster undo', () => {
  test('offers no undo while paying: Cancel rolls the whole spell back (#55)', () => {
    const sent = cluster({ interaction: interaction({ mode: 'payMana', secondaryButtons: [CANCEL] }), undoable: true });
    expect(screen.queryByRole('button', { name: /Undo tap/ })).toBeNull();
    fireEvent.keyDown(window, { key: 'z', ctrlKey: true });
    expect(sent).toEqual([]);
  });

  test('takes back mana tapped at priority while the stack is empty', () => {
    const sent = cluster({ interaction: interaction({}), undoable: true });
    fireEvent.click(screen.getByRole('button', { name: /Undo tap/ }));
    expect(sent).toEqual([{ type: 'action', action: 'UNDO' }]);
  });

  test('has nothing to undo without a saved point', () => {
    cluster({ interaction: interaction({}), undoable: false });
    expect(screen.queryByRole('button', { name: /Undo tap/ })).toBeNull();
  });
});

describe('ActionCluster shortcuts', () => {
  const main = { label: 'OK', command: { type: 'boolean', value: true } as Command, tone: 'primary' as const };

  test('Space and Enter answer the main action even without a legacy shortcut label', () => {
    const sent = cluster({ interaction: interaction({ mainButton: main }) });
    fireEvent.keyDown(window, { key: ' ', code: 'Space' });
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(sent).toEqual([main.command, main.command]);
  });

  test.each(['Pay {1}?', 'Use the optional trigger?'])('Space and Enter do not answer a question: %s', async (headline) => {
    const user = userEvent.setup();
    const no = { ...main, label: 'No', command: { type: 'boolean', value: false } as Command };
    const sent = cluster({ interaction: interaction({ mode: 'question', headline, mainButton: no, secondaryButtons: [{ ...main, label: 'Yes' }] }) });
    await user.keyboard(' {Enter} {Enter}');
    expect(sent).toEqual([]);
    fireEvent.keyDown(window, { key: 'Alt', altKey: true });
    expect(screen.queryByText('Space / Enter')).toBeNull();
    expect(screen.getByRole('button', { name: 'No' }).getAttribute('aria-keyshortcuts')).toBeNull();
    await user.keyboard('{Alt>}{Enter}{/Alt}');
    expect(sent).toEqual([main.command]);
    // A deliberate click or Tab to the button still allows an answer.
    screen.getByRole('button', { name: 'No' }).focus();
    await user.keyboard('{Enter}');
    expect(sent).toEqual([main.command, no.command]);
  });

  test('a question can explicitly opt in to the primary shortcut', () => {
    const sent = cluster({ interaction: interaction({ mode: 'question', mainButton: { ...main, shortcut: 'Space' } }) });
    fireEvent.keyDown(window, { key: ' ', code: 'Space' });
    fireEvent.keyDown(window, { key: 'Enter' });
    expect(sent).toEqual([main.command, main.command]);
  });

  test('Alt chooses secondary actions, reveals hints and clears them on blur', () => {
    const sent = cluster({ interaction: interaction({ mainButton: main, secondaryButtons: [CANCEL] }) });
    fireEvent.keyDown(window, { key: 'Alt', altKey: true });
    expect(screen.getByText('Alt+1')).toBeTruthy();
    fireEvent.keyDown(window, { key: 'Enter', altKey: true });
    fireEvent.keyDown(window, { key: '¡', code: 'Digit1', altKey: true });
    fireEvent.keyDown(window, { key: ' ', altKey: true });
    expect(sent).toEqual([CANCEL.command, CANCEL.command]);
    fireEvent.blur(window);
    expect(screen.queryByText('Alt+1')).toBeNull();
  });

  test('does not pass from focused cards, typing, repeats, menus or other dialogs', () => {
    const sent = cluster({ interaction: interaction({ mainButton: main }) });
    const { container } = render(<><textarea /><div role="listitem" tabIndex={0}>Card</div></>);
    fireEvent.keyDown(container.querySelector('textarea')!, { key: 'Enter' });
    fireEvent.keyDown(screen.getByRole('listitem'), { key: 'Enter' });
    fireEvent.keyDown(window, { key: 'Enter', repeat: true });
    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    const dialog = render(<div role="dialog" />);
    fireEvent.keyDown(window, { key: 'Enter' });
    dialog.unmount();
    const menu = render(<div role="menu" />);
    fireEvent.keyDown(window, { key: 'Enter' });
    menu.unmount();
    expect(sent).toEqual([]);
  });

  test('Alt+Enter on a focused object uses only the secondary action', () => {
    const sent = cluster({ interaction: interaction({ mainButton: main, secondaryButtons: [CANCEL] }) });
    let activated = false;
    render(<div role="button" tabIndex={0} onKeyDown={() => { activated = true; }}>Target</div>);
    fireEvent.keyDown(screen.getByRole('button', { name: 'Target' }), { key: 'Enter', altKey: true });
    expect(sent).toEqual([CANCEL.command]);
    expect(activated).toBe(false);
  });

  test('Alt+Enter never falls back to the focused primary button', async () => {
    const user = userEvent.setup();
    const sent = cluster({ interaction: interaction({ mainButton: main }) });
    screen.getByRole('button', { name: 'OK' }).focus();
    await user.keyboard('{Alt>}{Enter}{/Alt}');
    expect(sent).toEqual([]);
    await user.keyboard('{Enter}');
    expect(sent).toEqual([main.command]);
  });

  test('waits for the server, but permits payment cancellation with Escape or Alt', () => {
    const sent: Command[] = [];
    render(<ActionCluster interaction={interaction({ mode: 'payMana', mainButton: main, secondaryButtons: [CANCEL] })} awaiting status={null} canAct special={false} holdingPriority={false} onCommand={(command) => sent.push(command)} />);
    fireEvent.keyDown(window, { key: 'Enter' });
    fireEvent.keyDown(window, { key: 'Enter', altKey: true });
    fireEvent.keyDown(window, { key: '1', altKey: true });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(sent).toEqual([CANCEL.command, CANCEL.command, CANCEL.command]);
  });

  test('only cancellation can interrupt a pending payment', () => {
    const sent: Command[] = [];
    render(<ActionCluster interaction={interaction({ mode: 'payMana', mainButton: main, secondaryButtons: [main, CANCEL] })} awaiting status={null} canAct special={false} holdingPriority={false} onCommand={(command) => sent.push(command)} />);
    fireEvent.keyDown(window, { key: 'Enter', altKey: true });
    fireEvent.keyDown(window, { key: '2', altKey: true });
    expect(sent).toEqual([CANCEL.command]);
  });
});

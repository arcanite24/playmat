import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
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

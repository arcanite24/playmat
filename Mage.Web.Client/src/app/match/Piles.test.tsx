// @vitest-environment happy-dom
import { render } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import type { CommanderStatus } from '../../core/game/commander';
import type { PlayerView } from '../../protocol/generated/views';
import { MiniPiles } from './Piles';

const player: PlayerView = {
  playerId: 'p2',
  name: 'Kismet',
  libraryCount: 84,
  graveyard: {},
  exile: {},
  commandList: [],
};

function commander(zone: CommanderStatus['zone'], tax = 0): CommanderStatus {
  return {
    id: 'cmd-1',
    name: "Atraxa, Praetors' Voice",
    ownerId: 'p2',
    zone,
    casts: tax / 2,
    tax,
    damage: [],
    card: { id: 'cmd-1', name: "Atraxa, Praetors' Voice" },
  };
}

describe('MiniPiles', () => {
  test('keeps enemy commander chips visible when the commander enters the battlefield, graveyard or exile', () => {
    const { getByRole, rerender } = render(<MiniPiles player={player} commanders={[commander('command', 2)]} />);
    const inCommand = getByRole('button', { name: /Atraxa, Praetors' Voice is in the command zone, costs 2 more/i });
    expect(inCommand.textContent).toContain('Atraxa');
    expect(inCommand.textContent).toContain('+2');

    rerender(<MiniPiles player={player} commanders={[commander('battlefield', 2)]} />);
    const onBoard = getByRole('button', { name: /Atraxa, Praetors' Voice is on the battlefield/i });
    expect(onBoard.textContent).toContain('Atraxa');
    expect(onBoard.textContent).not.toContain('+2');

    rerender(<MiniPiles player={player} commanders={[commander('graveyard', 2)]} />);
    expect(getByRole('button', { name: /Atraxa, Praetors' Voice is in the graveyard/i })).toBeTruthy();

    rerender(<MiniPiles player={player} commanders={[commander('exile', 2)]} />);
    expect(getByRole('button', { name: /Atraxa, Praetors' Voice is in exile/i })).toBeTruthy();
  });
});

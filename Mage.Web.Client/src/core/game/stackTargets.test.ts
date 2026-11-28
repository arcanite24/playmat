import { describe, expect, it } from 'vitest';
import type { CardView, GameView } from '../../protocol/generated/views';
import { describeStackTargets } from './stackTargets';

const VIEW: GameView = {
  myPlayerId: 'me',
  players: [
    { playerId: 'me', name: 'Brandon', battlefield: { c1: { id: 'c1', name: 'Llanowar Elves' } } },
    { playerId: 'ai1', name: 'Alice', battlefield: { c2: { id: 'c2', name: 'Grizzly Bears' } } },
    { playerId: 'ai2', name: 'Bob', battlefield: {} },
  ],
  stack: {},
};

describe('describeStackTargets', () => {
  it('pairs each chosen mode of a modal spell with its target in order', () => {
    const command: CardView = {
      id: 's1',
      name: "Kolaghan's Command",
      targets: ['me', 'ai2'],
      rules: [
        'Choose two —',
        "<span color='green'><i>Chosen mode: Target player discards a card</i></span>",
        "<span color='green'><i>Chosen mode: Kolaghan's Command deals 2 damage to any target</i></span>",
      ],
    };
    const summary = describeStackTargets(command, VIEW);
    expect(summary.hasModes).toBe(true);
    expect(summary.lines).toEqual([
      { index: 1, effect: 'Target player discards a card', targetId: 'me', targetName: 'Brandon (you)' },
      { index: 2, effect: "Kolaghan's Command deals 2 damage to any target", targetId: 'ai2', targetName: 'Bob' },
    ]);
  });

  it('keeps non-targeting chosen modes alongside targeting ones', () => {
    const spell: CardView = {
      id: 's2',
      name: 'Cryptic Command',
      targets: ['c2'],
      rules: [
        "<span color='green'><i>Chosen mode: Return target permanent to its owner's hand</i></span>",
        "<span color='green'><i>Chosen mode: Draw a card</i></span>",
      ],
    };
    const summary = describeStackTargets(spell, VIEW);
    expect(summary.lines).toEqual([
      { index: 1, effect: "Return target permanent to its owner's hand", targetId: 'c2', targetName: 'Grizzly Bears (Alice)' },
      { index: null, effect: 'Draw a card', targetId: null, targetName: null },
    ]);
  });

  it('pairs non-modal multi-effect spells targeting different players', () => {
    const explosion: CardView = {
      id: 's3',
      name: 'Explosion',
      targets: ['me', 'ai1'],
      rules: ['Explosion deals X damage to any target and target player draws X cards.'],
    };
    const summary = describeStackTargets(explosion, VIEW);
    expect(summary.lines).toEqual([
      { index: 1, effect: 'Explosion deals X damage to any target', targetId: 'me', targetName: 'Brandon (you)' },
      { index: 2, effect: 'target player draws X cards', targetId: 'ai1', targetName: 'Alice' },
    ]);
  });

  it('falls back to numbered targets without guessing when counts differ', () => {
    const multi: CardView = {
      id: 's4',
      name: 'Hex',
      targets: ['c1', 'c2'],
      rules: ['Destroy six target creatures.'],
    };
    const summary = describeStackTargets(multi, VIEW);
    expect(summary.lines).toEqual([
      { index: 1, effect: null, targetId: 'c1', targetName: 'Llanowar Elves (yours)' },
      { index: 2, effect: null, targetId: 'c2', targetName: 'Grizzly Bears (Alice)' },
    ]);
  });
});

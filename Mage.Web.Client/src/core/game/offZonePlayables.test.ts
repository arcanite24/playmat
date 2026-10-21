import { describe, expect, test } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { offZonePlayables } from './offZonePlayables';

const view = {
  myPlayerId: 'me',
  myHand: { h1: { id: 'h1', name: 'Bonecrusher Giant' } },
  players: [
    {
      playerId: 'me',
      graveyard: { f1: { id: 'f1', name: 'Faithless Looting' } },
      exile: { a1: { id: 'a1', name: 'Bonecrusher Giant' } },
      battlefield: { p1: { id: 'p1', name: 'Mountain' } },
    },
    { playerId: 'them', graveyard: { o1: { id: 'o1', name: 'Shock' } }, exile: {} },
  ],
  exiles: [{ x1: { id: 'x1', name: 'Lightning Bolt' } }],
} as unknown as GameView;

describe('offZonePlayables', () => {
  test('an adventurer in exile and a flashback card in the graveyard can be played', () => {
    expect(offZonePlayables(view, ['h1', 'p1', 'a1', 'f1'])).toEqual([
      { card: { id: 'a1', name: 'Bonecrusher Giant' }, zone: 'exile' },
      { card: { id: 'f1', name: 'Faithless Looting' }, zone: 'graveyard' },
    ]);
  });

  test('cards in other exile zones and graveyards count when the server lists them', () => {
    expect(offZonePlayables(view, ['x1', 'o1']).map((entry) => [entry.card.id, entry.zone])).toEqual([['x1', 'exile'], ['o1', 'graveyard']]);
  });

  test('hand, battlefield and unknown ids are not off-zone', () => {
    expect(offZonePlayables(view, ['h1', 'p1', 'nope'])).toEqual([]);
    expect(offZonePlayables(null, ['a1'])).toEqual([]);
  });
});

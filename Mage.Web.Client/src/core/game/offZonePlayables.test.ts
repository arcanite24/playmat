import { describe, expect, test } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { heldOffZone, offZonePlayables } from './offZonePlayables';

const view = {
  myPlayerId: 'me',
  myHand: { h1: { id: 'h1', name: 'Bonecrusher Giant' } },
  players: [
    {
      playerId: 'me',
      graveyard: { f1: { id: 'f1', name: 'Faithless Looting' } },
      exile: { a1: { id: 'a1', name: 'Bonecrusher Giant' } },
      battlefield: { p1: { id: 'p1', name: 'Mountain' } },
      topCard: { id: 't1', name: 'Grizzly Bears' },
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

  test('the top card of the library can be cast when the server lists it (Realmwalker)', () => {
    expect(offZonePlayables(view, ['t1'])).toEqual([{ card: { id: 't1', name: 'Grizzly Bears' }, zone: 'library' }]);
  });

  test('hand, battlefield and unknown ids are not off-zone', () => {
    expect(offZonePlayables(view, ['h1', 'p1', 'nope'])).toEqual([]);
    expect(offZonePlayables(null, ['a1'])).toEqual([]);
  });

  test('between prompts the last playables stay while they are still in their zone (#170)', () => {
    const last = offZonePlayables(view, ['a1', 'f1']);
    expect(heldOffZone(last, view).map((entry) => entry.card.id)).toEqual(['a1', 'f1']);
    // the adventurer was cast: it left exile, so it leaves the hand too
    const cast = { ...view, players: [{ ...view.players![0], exile: {} }, view.players![1]] } as GameView;
    expect(heldOffZone(last, cast).map((entry) => entry.card.id)).toEqual(['f1']);
    expect(heldOffZone([], view)).toEqual([]);
  });
});

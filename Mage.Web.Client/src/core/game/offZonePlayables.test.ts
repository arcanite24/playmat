import { describe, expect, test } from 'vitest';
import type { GameView } from '../../protocol/generated/views';
import { NO_OFF_ZONE_HOLD, nextOffZoneHold, offZonePlayables } from './offZonePlayables';

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

  const main = { ...view, turn: 3, step: 'PRECOMBAT_MAIN' } as GameView;
  const decide = (live: string[]) => nextOffZoneHold(NO_OFF_ZONE_HOLD, offZonePlayables(main, live), main);

  test('between prompts the last playables stay while they are still in their zone (#170)', () => {
    const last = decide(['a1', 'f1']);
    expect(nextOffZoneHold(last, null, main).ids).toEqual(['a1', 'f1']);
    expect(nextOffZoneHold(last, null, main)).toBe(last);
    // the adventurer was cast: it left exile, so it leaves the hand too
    const cast = { ...main, players: [{ ...main.players![0], exile: {} }, main.players![1]] } as GameView;
    expect(nextOffZoneHold(last, null, cast).ids).toEqual(['f1']);
    expect(nextOffZoneHold(NO_OFF_ZONE_HOLD, null, main)).toBe(NO_OFF_ZONE_HOLD);
  });

  test('a list in the same step that leaves an exile card out does not drop it from the hand (#248)', () => {
    const last = decide(['h1', 'a1']);
    // a prompt or update in the same main phase lists only the hand card
    const partial = nextOffZoneHold(last, offZonePlayables(main, ['h1']), main);
    expect(partial).toBe(last);
    // a new playable from the graveyard joins after it, so the exile card keeps its place
    expect(nextOffZoneHold(last, offZonePlayables(main, ['f1']), main).ids).toEqual(['a1', 'f1']);
  });

  test('a priority prompt in a new step decides again', () => {
    const last = decide(['a1']);
    const combat = { ...main, step: 'DECLARE_ATTACKERS' } as GameView;
    // waiting through the step change keeps it until a prompt decides
    expect(nextOffZoneHold(last, null, combat).ids).toEqual(['a1']);
    expect(nextOffZoneHold(last, offZonePlayables(combat, ['h1']), combat)).toEqual({ ids: [], turn: 3, step: 'DECLARE_ATTACKERS' });
  });
});

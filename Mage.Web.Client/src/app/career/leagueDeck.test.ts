import { describe, expect, test } from 'vitest';
import { missingCards } from './leagueModel';

const entry = (cardName: string, amount: number) => ({ cardName, setCode: 'TST', cardNumber: '1', amount });

describe('missingCards', () => {
  test('counts by name across printings and zones, up to a playset, basics free', () => {
    const deck = { cards: [entry('Sheoldred', 3), entry('Forest', 20), entry('Opt', 2)], sideboard: [entry('Sheoldred', 2)] };
    const owned = new Map([['sheoldred', 1], ['opt', 2]]);
    expect(missingCards(deck, owned)).toEqual([{ name: 'Sheoldred', inDeck: 4, owned: 1 }]);
  });
});

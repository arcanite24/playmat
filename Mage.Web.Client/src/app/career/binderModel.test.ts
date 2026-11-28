import { describe, expect, test } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { binderCards, binderPages, collectionNames, collectionPlaysetCount, filterMissingPlayset, isMissingPlayset, newInBinder, playsetProgress } from './binderModel';

const card = (name: string, cardNumber: string, rarity = 'COMMON'): CardView => ({ name, cardNumber, expansionSetCode: 'THS', rarity } as CardView);

describe('binder', () => {
  test('one pocket per name in collector-number order, basics left out', () => {
    const cards = binderCards([card('Omenspeaker', '57'), card('Anthousa', '149', 'RARE'), card('Plains', '230'), card('Akroan Crusader', '111'), card('Omenspeaker', '9a')]);
    expect(cards.map((item) => `${item.cardNumber} ${item.name}`)).toEqual(['9a Omenspeaker', '111 Akroan Crusader', '149 Anthousa']);
    expect(cards[2].rarity).toBe('rare');
  });

  test('cuts cards into pages of nine', () => {
    const pages = binderPages(Array.from({ length: 20 }, (_, i) => i));
    expect(pages.map((page) => page.length)).toEqual([9, 9, 2]);
    expect(binderPages([])).toEqual([]);
  });

  test('new cards shine after the first look, never on it', () => {
    const names = collectionNames([{ name: 'Anthousa', count: 1 }, { name: 'Omenspeaker', count: 0 }]);
    expect(names).toEqual(['anthousa']);
    expect(newInBinder(null, names).fresh.size).toBe(0);
    expect([...newInBinder(['anthousa'], ['anthousa', 'akroan crusader']).fresh]).toEqual(['akroan crusader']);
  });

  test('missing playset filter and playset counters ignore basic lands (#408)', () => {
    const owned = new Map<string, number>([
      ['lightning bolt', 4],
      ['shock', 2],
      ['plains', 1],
    ]);
    expect(isMissingPlayset('Lightning Bolt', owned)).toBe(false);
    expect(isMissingPlayset('Shock', owned)).toBe(true);
    expect(isMissingPlayset('Opt', owned)).toBe(true);
    expect(isMissingPlayset('Plains', owned)).toBe(false);

    const setCards = [{ name: 'Lightning Bolt' }, { name: 'Shock' }, { name: 'Opt' }, { name: 'Plains' }];
    expect(filterMissingPlayset(setCards, owned).map((item) => item.name)).toEqual(['Shock', 'Opt']);
    expect(playsetProgress(setCards, owned)).toEqual({ playsets: 1, total: 3 });
    expect(collectionPlaysetCount(owned)).toBe(1);
  });
});

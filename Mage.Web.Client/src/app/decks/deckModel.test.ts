import { describe, expect, it } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import type { DeckCardLists } from '../../core/decks/types';
import { addCard, changePrinting, copiesByName, copyLimit, countZone, entryKey, groupDeck, manaColumns, manaCurve, moveCard, removeCard } from './deckModel';

const empty: DeckCardLists = { name: 'Test', cards: [], sideboard: [] };
const bolt = { cardName: 'Lightning Bolt', setCode: 'M10', cardNumber: '146' };
const boltAlt = { cardName: 'Lightning Bolt', setCode: '2XM', cardNumber: '129' };
const mountain = { cardName: 'Mountain', setCode: 'M21', cardNumber: '270' };

describe('deck editing', () => {
  it('adds copies to the same printing and keeps printings apart', () => {
    let deck = addCard(empty, 'cards', bolt);
    deck = addCard(deck, 'cards', bolt);
    deck = addCard(deck, 'cards', boltAlt);
    expect(deck.cards).toHaveLength(2);
    expect(countZone(deck, 'cards')).toBe(3);
    expect(copiesByName(deck, 'lightning bolt')).toBe(3);
  });

  it('removes and drops empty entries', () => {
    const deck = removeCard(addCard(empty, 'cards', bolt), 'cards', entryKey(bolt));
    expect(deck.cards).toHaveLength(0);
  });

  it('switches an entry to another printing, merging copies', () => {
    const deck = addCard(addCard(empty, 'cards', bolt, 3), 'cards', boltAlt, 1);
    const changed = changePrinting(deck, 'cards', entryKey(bolt), boltAlt);
    expect(changed.cards).toEqual([{ ...boltAlt, amount: 4 }]);
    expect(changePrinting(deck, 'cards', entryKey(bolt), bolt)).toBe(deck);
  });

  it('moves cards between main deck and sideboard', () => {
    const deck = moveCard(addCard(empty, 'cards', bolt, 3), 'cards', entryKey(bolt), 2);
    expect(countZone(deck, 'cards')).toBe(1);
    expect(countZone(deck, 'sideboard')).toBe(2);
  });
});

describe('copyLimit', () => {
  it('allows four of a card, any number of basics and of cards that say so', () => {
    expect(copyLimit(undefined, 'Lightning Bolt')).toBe(4);
    expect(copyLimit(undefined, 'Mountain')).toBe(Infinity);
    expect(copyLimit({ rules: ['A deck can have any number of cards named Relentless Rats.'] }, 'Relentless Rats')).toBe(Infinity);
    expect(copyLimit({ rules: ['A deck can have up to seven cards named Seven Dwarves.'] }, 'Seven Dwarves')).toBe(7);
  });

  it('allows one copy in singleton formats, basics aside', () => {
    expect(copyLimit(undefined, 'Sol Ring', true)).toBe(1);
    expect(copyLimit(undefined, 'Forest', true)).toBe(Infinity);
    expect(copyLimit({ rules: ['A deck can have any number of cards named Relentless Rats.'] }, 'Relentless Rats', true)).toBe(Infinity);
  });
});

describe('grouping and curve', () => {
  const info = new Map<string, CardView>([
    [entryKey(bolt), { name: 'Lightning Bolt', cardTypes: ['INSTANT'], manaValue: 1 }],
    [entryKey(mountain), { name: 'Mountain', cardTypes: ['LAND'], manaValue: 0 }],
  ]);
  const deck = addCard(addCard(addCard(empty, 'cards', bolt, 4), 'cards', mountain, 20), 'cards', { cardName: 'Mystery', setCode: '', cardNumber: '' });

  it('groups by type with unknown cards last', () => {
    expect(groupDeck(deck.cards, info).map((group) => [group.key, group.count])).toEqual([['spells', 4], ['lands', 20], ['unknown', 1]]);
  });

  it('counts nonland cards by mana value', () => {
    expect(manaCurve(deck.cards, info)).toEqual([0, 4, 0, 0, 0, 0, 0, 0]);
  });
});

describe('mana value columns', () => {
  it('puts 0 and 1 together, 6 and up together, lands apart, and counts copies', () => {
    const card = (name: string, manaValue: number, types: CardView['cardTypes']) => ({ name, manaValue, cardTypes: types }) as CardView;
    const entries = [
      { cardName: 'Ornithopter', setCode: 'A', cardNumber: '1', amount: 1 },
      { cardName: 'Bolt', setCode: 'A', cardNumber: '2', amount: 2 },
      { cardName: 'Bear', setCode: 'A', cardNumber: '3', amount: 3 },
      { cardName: 'Dragon', setCode: 'A', cardNumber: '4', amount: 1 },
      { cardName: 'Mountain', setCode: 'A', cardNumber: '5', amount: 16 },
    ];
    const info = new Map([
      ['A:1', card('Ornithopter', 0, ['ARTIFACT', 'CREATURE'])],
      ['A:2', card('Bolt', 1, ['INSTANT'])],
      ['A:3', card('Bear', 2, ['CREATURE'])],
      ['A:4', card('Dragon', 7, ['CREATURE'])],
      ['A:5', card('Mountain', 0, ['LAND'])],
    ]);
    const columns = manaColumns(entries, info);
    expect(columns.map((column) => [column.key, column.count])).toEqual([['0-1', 3], ['2', 3], ['3', 0], ['4', 0], ['5', 0], ['6+', 1], ['lands', 16]]);
    expect(columns[0].rows.map((row) => row.entry.cardName)).toEqual(['Ornithopter', 'Bolt']);
    expect(manaColumns([{ cardName: 'X', amount: 1 }], new Map()).at(-1)?.key).toBe('unknown');
  });
});

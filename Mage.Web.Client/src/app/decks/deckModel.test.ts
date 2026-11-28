import { describe, expect, it } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import type { DeckCardLists } from '../../core/decks/types';
import { groupRows } from './deckGroups';
import {
  addCard, categoryOf, changePrinting, chooseCover, copiesByName, copyLimit, countZone, deckCategories, entryKey, finalizeDeck, groupDeck, manaCurve,
  movableCopies, moveCard, removeCard, renameCategory, setCategory, toWire,
} from './deckModel';

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

  it('keeps a maybeboard that never counts toward the limit or reaches the server', () => {
    let deck = addCard(empty, 'cards', bolt, 4);
    deck = moveCard(deck, 'cards', entryKey(bolt), 1, 'maybeboard');
    expect(countZone(deck, 'cards')).toBe(3);
    expect(countZone(deck, 'maybeboard')).toBe(1);
    expect(copiesByName(deck, 'Lightning Bolt')).toBe(3);
    deck = addCard(deck, 'maybeboard', mountain);
    expect(toWire(deck)).not.toHaveProperty('maybeboard');
    expect(moveCard(deck, 'maybeboard', entryKey(bolt), 1, 'cards').cards).toEqual([{ ...bolt, amount: 4 }]);
    expect(moveCard(deck, 'cards', entryKey(bolt), 1, 'cards')).toBe(deck);
  });

  it('moves out of the maybeboard only within the copy limit', () => {
    const deck = addCard(addCard(empty, 'cards', bolt, 3), 'maybeboard', boltAlt, 6);
    const key = entryKey(boltAlt);
    expect(movableCopies(deck, 'maybeboard', key, 6, 'cards', { limit: 4 })).toBe(1);
    expect(movableCopies(deck, 'maybeboard', key, 6, 'sideboard', { limit: 3 })).toBe(0);
    expect(movableCopies(deck, 'maybeboard', key, 6, 'cards', { limit: Infinity })).toBe(6);
    // deck to sideboard leaves the count alone; into the maybeboard is always fine
    expect(movableCopies(deck, 'cards', entryKey(bolt), 3, 'sideboard', { limit: 3 })).toBe(3);
    expect(movableCopies(deck, 'cards', entryKey(bolt), 3, 'maybeboard', { limit: 0 })).toBe(3);
    expect(movableCopies(deck, 'cards', entryKey(bolt), 1, 'cards', { limit: 4 })).toBe(0);
  });

  it('keeps the command zone to cards that may command, while it has room', () => {
    const deck = addCard(addCard(empty, 'cards', mountain, 1), 'maybeboard', bolt, 1);
    const open = { limit: 1, command: { allowed: true, max: 1 } };
    expect(movableCopies(deck, 'cards', entryKey(mountain), 1, 'sideboard', { limit: 1, command: { allowed: false, max: 1 } })).toBe(0);
    expect(movableCopies(deck, 'maybeboard', entryKey(bolt), 1, 'sideboard', open)).toBe(1);
    const full = moveCard(deck, 'cards', entryKey(mountain), 1, 'sideboard');
    expect(movableCopies(full, 'maybeboard', entryKey(bolt), 1, 'sideboard', open)).toBe(0);
    // out of the command zone is always fine
    expect(movableCopies(full, 'sideboard', entryKey(mountain), 1, 'cards', open)).toBe(1);
  });
});

describe('categories', () => {
  it('sets, renames and clears a category by card name', () => {
    let deck = addCard(addCard(empty, 'cards', bolt, 4), 'maybeboard', mountain);
    deck = setCategory(deck, 'Lightning Bolt', ' Removal ');
    deck = setCategory(deck, 'Mountain', 'Mana');
    deck = setCategory(deck, 'Gone', 'Old');
    expect(categoryOf(deck, 'lightning bolt')).toBe('Removal');
    expect(deckCategories(deck)).toEqual(['Mana', 'Removal']);
    deck = renameCategory(deck, 'Removal', 'Mana');
    expect(deckCategories(deck)).toEqual(['Mana']);
    deck = setCategory(setCategory(setCategory(deck, 'Lightning Bolt', null), 'Mountain', ''), 'Gone', null);
    expect(deck.categories).toBeUndefined();
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

describe('view groups', () => {
  const card = (name: string, manaValue: number, types: CardView['cardTypes'], extra: Partial<CardView> = {}) => ({ name, manaValue, cardTypes: types, ...extra }) as CardView;
  const entries = [
    { cardName: 'Ornithopter', setCode: 'A', cardNumber: '1', amount: 1 },
    { cardName: 'Bolt', setCode: 'A', cardNumber: '2', amount: 2 },
    { cardName: 'Bear', setCode: 'A', cardNumber: '3', amount: 3 },
    { cardName: 'Dragon', setCode: 'A', cardNumber: '4', amount: 1 },
    { cardName: 'Mountain', setCode: 'A', cardNumber: '5', amount: 16 },
  ];
  const info = new Map([
    ['A:1', card('Ornithopter', 0, ['ARTIFACT', 'CREATURE'], { rarity: 'UNCOMMON' })],
    ['A:2', card('Bolt', 1, ['INSTANT'], { color: { red: true }, rarity: 'COMMON' })],
    ['A:3', card('Bear', 2, ['CREATURE'], { color: { green: true }, rarity: 'COMMON' })],
    ['A:4', card('Dragon', 7, ['CREATURE'], { color: { red: true, green: true }, rarity: 'MYTHIC' })],
    ['A:5', card('Mountain', 0, ['LAND'], { rarity: 'LAND' })],
  ]);
  const shape = (groupBy: Parameters<typeof groupRows>[2]['groupBy'], extra = {}) =>
    groupRows(entries, info, { groupBy, sortBy: 'mana', ...extra }).map((group) => [group.key, group.count]);

  it('lays out mana value columns: 0 and 1 together, 6 and up together, lands apart', () => {
    expect(shape('mana', { keepEmpty: true })).toEqual([['0-1', 3], ['2', 3], ['3', 0], ['4', 0], ['5', 0], ['6+', 1], ['lands', 16]]);
    expect(shape('mana')).toEqual([['0-1', 3], ['2', 3], ['6+', 1], ['lands', 16]]);
    const columns = groupRows(entries, info, { groupBy: 'mana', sortBy: 'mana' });
    expect(columns[0].rows.map((row) => row.entry.cardName)).toEqual(['Ornithopter', 'Bolt']);
    expect(groupRows([{ cardName: 'X', amount: 1 }], new Map(), { groupBy: 'mana', sortBy: 'mana' }).at(-1)?.key).toBe('unknown');    // empty costs only between the cheapest and dearest the deck plays
    const cheap = entries.filter((entry) => entry.cardName !== 'Dragon');
    expect(groupRows(cheap, info, { groupBy: 'mana', sortBy: 'mana', keepEmpty: true }).map((group) => group.key)).toEqual(['0-1', '2', 'lands']);
  });

  it('groups by type, color and rarity', () => {
    expect(shape('type')).toEqual([['creatures', 5], ['spells', 2], ['lands', 16]]);
    expect(shape('color')).toEqual([['R', 2], ['G', 3], ['multi', 1], ['colorless', 1], ['lands', 16]]);
    expect(shape('rarity')).toEqual([['MYTHIC', 1], ['UNCOMMON', 1], ['COMMON', 5], ['LAND', 16]]);
    expect(shape('none')).toEqual([['all', 23]]);
  });

  it("puts the player's categories first and the rest by type", () => {
    const groups = groupRows(entries, info, { groupBy: 'category', sortBy: 'name', categories: { bolt: 'Removal', dragon: 'Finishers' } });
    expect(groups.map((group) => [group.label, group.count, group.category])).toEqual([
      ['Finishers', 1, 'Finishers'], ['Removal', 2, 'Removal'], ['Creatures', 4, undefined], ['Lands', 16, undefined],
    ]);
    expect(groups[2].rows.map((row) => row.entry.cardName)).toEqual(['Bear', 'Ornithopter']);
  });
});

describe('deck cover (#167)', () => {
  const titan = { cardName: 'Inferno Titan', setCode: 'M11', cardNumber: '146' };
  const card = (name: string, manaValue: number, types = ['INSTANT']) => ({ name, manaValue, cardTypes: types } as unknown as CardView);
  const info = new Map<string, CardView>([
    [entryKey(bolt), card('Lightning Bolt', 1)],
    [entryKey(boltAlt), card('Lightning Bolt', 1)],
    [entryKey(titan), card('Inferno Titan', 6, ['CREATURE'])],
    [entryKey(mountain), card('Mountain', 0, ['LAND'])],
  ]);
  const burn: DeckCardLists = { name: 'Burn', cards: [{ ...bolt, amount: 4 }, { ...titan, amount: 1 }, { ...mountain, amount: 20 }], sideboard: [] };

  it('picks the costliest nonland card on its own', () => {
    const deck = finalizeDeck(burn, info);
    expect(deck.coverCard).toEqual({ setCode: 'M11', cardNumber: '146', name: 'Inferno Titan' });
    expect(deck.coverPinned).toBeUndefined();
  });

  it("follows a printing change when the cover is automatic (the bug in #167's comments)", () => {
    const auto = finalizeDeck({ ...burn, cards: [{ ...bolt, amount: 4 }] }, info);
    expect(auto.coverCard?.setCode).toBe('M10');
    const reprinted = finalizeDeck(changePrinting(auto, 'cards', entryKey(bolt), boltAlt), info);
    expect(reprinted.coverCard).toEqual({ setCode: '2XM', cardNumber: '129', name: 'Lightning Bolt' });
  });

  it("keeps the player's choice, printing and all, while its card is in the deck", () => {
    const chosen = chooseCover(burn, bolt, info);
    expect(chosen.coverCard).toEqual({ setCode: 'M10', cardNumber: '146', name: 'Lightning Bolt' });
    expect(chosen.coverPinned).toBe(true);
    // a costlier card doesn't take over
    expect(finalizeDeck(addCard(chosen, 'cards', { cardName: 'Big', setCode: 'X', cardNumber: '1' }), info).coverCard?.name).toBe('Lightning Bolt');
    // another printing for the cover than the deck plays stays too
    const otherArt = chooseCover(burn, boltAlt, info);
    expect(finalizeDeck(otherArt, info).coverCard).toEqual({ setCode: '2XM', cardNumber: '129', name: 'Lightning Bolt' });
    // the deck switching printings leaves the chosen one alone
    expect(finalizeDeck(changePrinting(chosen, 'cards', entryKey(bolt), boltAlt), info).coverCard?.setCode).toBe('M10');
  });

  it('falls back to an automatic cover when the chosen card leaves the deck', () => {
    const chosen = chooseCover(burn, bolt, info);
    const without = finalizeDeck(removeCard(chosen, 'cards', entryKey(bolt), 4), info);
    expect(without.coverCard?.name).toBe('Inferno Titan');
    expect(without.coverPinned).toBeUndefined();
  });

  it('goes back to automatic on request', () => {
    const chosen = chooseCover(burn, bolt, info);
    const auto = chooseCover(chosen, null, info);
    expect(auto.coverCard?.name).toBe('Inferno Titan');
    expect(auto.coverPinned).toBeUndefined();
  });

  it('a cover in the command zone counts as in the deck', () => {
    const commander = { cardName: 'Krenko, Mob Boss', setCode: 'M13', cardNumber: '139' };
    const deck = chooseCover({ ...burn, sideboard: [{ ...commander, amount: 1 }] }, commander, info);
    expect(finalizeDeck(deck, info).coverCard?.name).toBe('Krenko, Mob Boss');
  });
});

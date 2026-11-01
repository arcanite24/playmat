import { describe, expect, test } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { EMPTY_FILTERS, isFiltered, matchesFilters, type CardFilters } from './filters';

const bolt = {
  name: 'Lightning Bolt', cardTypes: ['INSTANT'], subTypes: [], rules: ['Lightning Bolt deals 3 damage to any target.'],
  color: { red: true }, manaValue: 1, rarity: 'COMMON', expansionSetCode: 'M10',
} as unknown as CardView;
const titan = {
  name: 'Wurmcoil Engine', cardTypes: ['ARTIFACT', 'CREATURE'], subTypes: ['Phyrexian', 'Wurm'], rules: ['Deathtouch, lifelink'],
  color: {}, manaValue: 6, rarity: 'MYTHIC', expansionSetCode: 'SOM',
} as unknown as CardView;
const dragon = {
  name: 'Nicol Bolas, the Ravager', cardTypes: ['CREATURE'], subTypes: ['Elder', 'Dragon'], rules: ['Flying'],
  color: { blue: true, black: true, red: true }, manaValue: 7, rarity: 'MYTHIC', expansionSetCode: 'M19',
} as unknown as CardView;
const cards = [bolt, titan, dragon];

const pick = (patch: Partial<CardFilters>) => cards.filter((card) => matchesFilters(card, { ...EMPTY_FILTERS, ...patch })).map((card) => card.name);

describe('matchesFilters', () => {
  test('no filters keep every card', () => {
    expect(pick({})).toHaveLength(3);
    expect(isFiltered(EMPTY_FILTERS)).toBe(false);
  });

  test('text matches names, or types and rules text too when asked', () => {
    expect(pick({ text: 'bol' })).toEqual(['Lightning Bolt', 'Nicol Bolas, the Ravager']);
    expect(pick({ text: 'dragon' })).toEqual([]);
    expect(pick({ text: 'dragon', anyText: true })).toEqual(['Nicol Bolas, the Ravager']);
    expect(pick({ text: 'lifelink', anyText: true })).toEqual(['Wurmcoil Engine']);
  });

  test('colors match any chosen color, and colorless means no color', () => {
    expect(pick({ colors: ['red'] })).toEqual(['Lightning Bolt', 'Nicol Bolas, the Ravager']);
    expect(pick({ colors: ['colorless'] })).toEqual(['Wurmcoil Engine']);
    expect(pick({ colors: ['blue', 'colorless'] })).toEqual(['Wurmcoil Engine', 'Nicol Bolas, the Ravager']);
  });

  test('mana value is exact, and 7 means 7 or more', () => {
    expect(pick({ manaValue: 1 })).toEqual(['Lightning Bolt']);
    expect(pick({ manaValue: 7 })).toEqual(['Nicol Bolas, the Ravager']);
  });

  test('type, rarity and set narrow it down', () => {
    expect(pick({ type: 'CREATURE' })).toEqual(['Wurmcoil Engine', 'Nicol Bolas, the Ravager']);
    expect(pick({ rarity: 'MYTHIC', type: 'ARTIFACT' })).toEqual(['Wurmcoil Engine']);
    expect(pick({ set: 'M10' })).toEqual(['Lightning Bolt']);
  });
});

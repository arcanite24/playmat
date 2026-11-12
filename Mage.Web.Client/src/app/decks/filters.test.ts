import { describe, expect, test } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import { EMPTY_FILTERS, intersectIdentity, isFiltered, matchesFilters, onlyColorsIdentity, type CardFilters } from './filters';

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
const helix = {
  name: 'Lightning Helix', cardTypes: ['INSTANT'], subTypes: [], rules: [], color: { white: true, red: true }, manaValue: 2,
  rarity: 'UNCOMMON', expansionSetCode: 'RAV', originalColorIdentity: '{R}{W}',
} as unknown as CardView;
const relic = {
  // colorless, but a red ability in its rules gives it a red identity
  name: 'Red Relic', cardTypes: ['ARTIFACT'], subTypes: [], rules: [], color: {}, manaValue: 2, rarity: 'COMMON',
  expansionSetCode: 'M10', originalColorIdentity: '{R}',
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

  test('only these colors keeps cards inside the chosen colors (#198)', () => {
    const more = [bolt, titan, dragon, helix, relic];
    const only = (colors: string[]) => more.filter((card) => matchesFilters(card, { ...EMPTY_FILTERS, colors, onlyColors: true })).map((card) => card.name);
    expect(only(['red'])).toEqual(['Lightning Bolt']);
    expect(only(['red', 'colorless'])).toEqual(['Lightning Bolt', 'Wurmcoil Engine', 'Red Relic']);
    expect(only(['white', 'colorless'])).toEqual(['Wurmcoil Engine']);
    expect(only(['white', 'red'])).toEqual(['Lightning Bolt', 'Lightning Helix']);
    expect(only(['colorless'])).toEqual(['Wurmcoil Engine']);
    // off: any chosen color
    expect(more.filter((card) => matchesFilters(card, { ...EMPTY_FILTERS, colors: ['white', 'colorless'] })).map((card) => card.name))
      .toEqual(['Wurmcoil Engine', 'Lightning Helix', 'Red Relic']);
  });
});

describe('identity limits for the server search', () => {
  test('only these colors is the chosen colors as letters', () => {
    expect(onlyColorsIdentity(['red', 'white', 'colorless'])).toEqual(['W', 'R']);
    expect(onlyColorsIdentity(['colorless'])).toEqual([]);
    expect(onlyColorsIdentity([])).toBeNull();
    expect(onlyColorsIdentity(['red'], false)).toBeNull();
  });

  test("a commander's identity and the chosen colors meet in the middle", () => {
    expect(intersectIdentity(null, null)).toBeNull();
    expect(intersectIdentity(['W', 'U'], null)).toEqual(['W', 'U']);
    expect(intersectIdentity(null, ['R'])).toEqual(['R']);
    expect(intersectIdentity(['W', 'U', 'B'], ['U', 'R'])).toEqual(['U']);
  });
});

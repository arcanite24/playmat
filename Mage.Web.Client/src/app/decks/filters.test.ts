import { describe, expect, test } from 'vitest';
import type { CardView } from '../../protocol/generated/views';
import {
  effectiveSetCodes, EMPTY_FILTERS, formatEraSummary, intersectIdentity, isFiltered, matchesFilters,
  onlyColorsIdentity, onlyInSidelinedSets, setReleaseYear, setsMatchingEra, type CardFilters, type SetOption,
} from './filters';

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

describe('onlyInSidelinedSets (#392)', () => {
  test('keeps the cards only joke and digital sets print, once each', () => {
    const lotus = { name: 'Black Lotus', expansionSetCode: 'YDMU' } as CardView;
    const returner = { name: 'Diminished Returner', expansionSetCode: 'YDMU' } as CardView;
    const main = [{ name: 'Black Lotus', expansionSetCode: 'LEA' } as CardView];
    expect(onlyInSidelinedSets(main, [lotus, returner, { ...returner, expansionSetCode: 'Y99' }])).toEqual([returner]);
    expect(onlyInSidelinedSets([], [])).toEqual([]);
  });
});

describe('era and multi-set filtering (#236)', () => {
  const catalog: SetOption[] = [
    { setCode: 'LEA', name: 'Limited Edition Alpha', releaseDate: Date.UTC(1993, 7, 5) },
    { setCode: 'TMP', name: 'Tempest', releaseDate: Date.UTC(1997, 9, 14) },
    { setCode: 'USG', name: "Urza's Saga", releaseDate: Date.UTC(1998, 9, 12) },
    { setCode: 'RAV', name: 'Ravnica: City of Guilds', releaseDate: Date.UTC(2005, 9, 7) },
    { setCode: 'M10', name: 'Magic 2010', releaseDate: Date.UTC(2009, 6, 17) },
    { setCode: 'SOM', name: 'Scars of Mirrodin', releaseDate: Date.UTC(2010, 9, 1) },
    { setCode: 'M19', name: 'Core Set 2019', releaseDate: Date.UTC(2018, 6, 13) },
  ];

  test('extracts UTC release years and filters sets by year range', () => {
    expect(setReleaseYear(catalog[0])).toBe(1993);
    expect(setReleaseYear({})).toBeNull();
    // Pre-1998 (<= 1997)
    expect(setsMatchingEra(catalog, null, 1997).map((s) => s.setCode)).toEqual(['LEA', 'TMP']);
    // 1997-2005 inclusive
    expect(setsMatchingEra(catalog, 1997, 2005).map((s) => s.setCode)).toEqual(['TMP', 'USG', 'RAV']);
    // Modern era (2003+)
    expect(setsMatchingEra(catalog, 2003, null).map((s) => s.setCode)).toEqual(['RAV', 'M10', 'SOM', 'M19']);
  });

  test('computes effectiveSetCodes for era, multi-set, and intersections', () => {
    expect(effectiveSetCodes(EMPTY_FILTERS, catalog)).toBeNull();
    expect(effectiveSetCodes({ ...EMPTY_FILTERS, yearFrom: 1997, yearTo: 2005 }, catalog)).toEqual(['TMP', 'USG', 'RAV']);
    expect(effectiveSetCodes({ ...EMPTY_FILTERS, sets: ['LEA', 'RAV'] }, catalog)).toEqual(['LEA', 'RAV']);
    // era + checked sets intersect cleanly
    expect(effectiveSetCodes({ ...EMPTY_FILTERS, yearFrom: 1997, yearTo: 2005, sets: ['LEA', 'RAV'] }, catalog)).toEqual(['RAV']);
    // impossible year range yields empty array
    expect(effectiveSetCodes({ ...EMPTY_FILTERS, yearFrom: 2020, yearTo: 1995 }, catalog)).toEqual([]);
  });

  test('matchesFilters honors era and multi-set filters and reports isFiltered', () => {
    const f = { ...EMPTY_FILTERS, yearFrom: 2005, yearTo: 2010 };
    expect(isFiltered(f)).toBe(true);
    expect([bolt, titan, dragon, helix].filter((c) => matchesFilters(c, f, catalog)).map((c) => c.name))
      .toEqual(['Lightning Bolt', 'Wurmcoil Engine', 'Lightning Helix']);
  });

  test('formats concise trigger labels for presets, custom ranges and set counts', () => {
    expect(formatEraSummary(EMPTY_FILTERS, catalog)).toBe('Era / Sets');
    expect(formatEraSummary({ ...EMPTY_FILTERS, yearTo: 1997 }, catalog)).toBe('Pre-1998');
    expect(formatEraSummary({ ...EMPTY_FILTERS, yearFrom: 1997, yearTo: 2005 }, catalog)).toBe('1997–2005');
    expect(formatEraSummary({ ...EMPTY_FILTERS, yearFrom: 1999, yearTo: 2002 }, catalog)).toBe('1999–2002');
    expect(formatEraSummary({ ...EMPTY_FILTERS, sets: ['TMP', 'USG'] }, catalog)).toBe('2 sets');
  });
});

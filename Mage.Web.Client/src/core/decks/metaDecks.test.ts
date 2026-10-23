import { describe, expect, test } from 'vitest';
import { metaDecksFor, pickMetaDeck, type MetaDeck } from './metaDecks';

const deck = (file: string, format: string, share: number): MetaDeck => ({
  file, name: file, format, tier: 'meta', share, cards: 60, cover: { name: file, setCode: 'SET', cardNumber: '1' }, source: '',
});

const DECKS = [
  deck('burn', 'Constructed - Modern', 5),
  deck('tron', 'Constructed - Modern', 15),
  deck('elves', 'Constructed - Pauper', 10),
];

describe('meta decks', () => {
  test('a format lists its own pool by recorded share', () => {
    expect(metaDecksFor(DECKS, 'Constructed - Modern').map((d) => d.file)).toEqual(['tron', 'burn']);
    expect(metaDecksFor(DECKS, 'Constructed - Freeform')).toEqual([]);
  });

  test('a pick is weighted by metagame share', () => {
    expect(pickMetaDeck(DECKS, 'Constructed - Modern', new Set(), () => 0)?.file).toBe('tron');
    expect(pickMetaDeck(DECKS, 'Constructed - Modern', new Set(), () => 0.74)?.file).toBe('tron');
    expect(pickMetaDeck(DECKS, 'Constructed - Modern', new Set(), () => 0.76)?.file).toBe('burn');
  });

  test('a pick skips taken decks and runs out', () => {
    expect(pickMetaDeck(DECKS, 'Constructed - Modern', new Set(['tron']), () => 0)?.file).toBe('burn');
    expect(pickMetaDeck(DECKS, 'Constructed - Modern', new Set(['tron', 'burn']))).toBeNull();
    expect(pickMetaDeck(DECKS, null)).toBeNull();
  });

  test('original rogue and fun lists remain reachable without a metagame share', () => {
    const pool: MetaDeck[] = [
      deck('meta', 'Constructed - Modern', 10),
      { ...deck('rogue', 'Constructed - Modern', 0), tier: 'rogue' },
      { ...deck('fun', 'Constructed - Modern', 0), tier: 'fun' },
    ];
    expect(pickMetaDeck(pool, 'Constructed - Modern', new Set(), () => 10.5 / 12)?.file).toBe('rogue');
    expect(pickMetaDeck(pool, 'Constructed - Modern', new Set(), () => 11.5 / 12)?.file).toBe('fun');
    expect(pickMetaDeck(pool, 'Constructed - Modern', new Set(['meta', 'rogue']), () => 0)?.file).toBe('fun');
  });
});

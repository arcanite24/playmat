import { describe, expect, it } from 'vitest';
import type { DeckCardLists, MetaFormat } from '../../protocol/generated/views';
import {
  adaptivePace, adaptiveSentence,
  adaptiveFormatFor, adaptiveSkill, agoParts, findMetaDeck, ghostOpponentId, ghostRecord, looksCommander, metaFormatsFor, metaOpponentId,
  MIRROR_OPPONENT_ID, normalizeGhostCode, opponentSource, tableMarkArgs, unseenCount,
} from './freePlay';

describe("the AI's deck source", () => {
  it('reads roster ids, meta files, ghost codes and the mirror', () => {
    expect(opponentSource(null)).toEqual({ kind: 'roster', id: null });
    expect(opponentSource('starter:mono-red.dck')).toEqual({ kind: 'roster', id: 'starter:mono-red.dck' });
    expect(opponentSource('a1b2')).toEqual({ kind: 'roster', id: 'a1b2' });
    expect(opponentSource(metaOpponentId('modern/burn.dck'))).toEqual({ kind: 'meta', file: 'modern/burn.dck' });
    expect(opponentSource(ghostOpponentId(' ab-cd 23 '))).toEqual({ kind: 'ghost', code: 'ABCD23' });
    expect(opponentSource(MIRROR_OPPONENT_ID)).toEqual({ kind: 'mirror' });
  });

  it('falls back to a random starter for an empty meta file or ghost code', () => {
    expect(opponentSource('meta:')).toEqual({ kind: 'roster', id: null });
    expect(opponentSource('ghost:  ')).toEqual({ kind: 'roster', id: null });
  });

  it('normalizes ghost codes the way the server does', () => {
    expect(normalizeGhostCode(' #k7m-p2q ')).toBe('K7MP2Q');
  });
});

describe('marking a free-play table', () => {
  it('marks ghost games and adaptive games, and leaves other tables alone', () => {
    expect(tableMarkArgs({ kind: 'roster', id: null }, null)).toBeNull();
    expect(tableMarkArgs({ kind: 'mirror' }, null)).toBeNull();
    expect(tableMarkArgs({ kind: 'ghost', code: 'ABC234' }, null)).toEqual(['ABC234', null]);
    expect(tableMarkArgs({ kind: 'meta', file: 'x.dck' }, 'constructed')).toEqual([null, 'constructed']);
    expect(tableMarkArgs({ kind: 'ghost', code: 'ABC234' }, 'commander')).toEqual(['ABC234', 'commander']);
  });

  it('rates commander games apart from the rest', () => {
    expect(adaptiveFormatFor(true)).toBe('commander');
    expect(adaptiveFormatFor(false)).toBe('constructed');
  });

  it('keeps an adaptive skill in the AI range, with a fallback when there is none', () => {
    expect(adaptiveSkill(6.4, 4)).toBe(6);
    expect(adaptiveSkill(14, 4)).toBe(10);
    expect(adaptiveSkill(0, 4)).toBe(1);
    expect(adaptiveSkill(undefined, 4)).toBe(4);
  });
});

describe('deck shapes', () => {
  const deck = (main: number, side: number): DeckCardLists => ({
    cards: [{ cardName: 'Forest', amount: main }],
    sideboard: side ? [{ cardName: 'Commander', amount: side }] : [],
  } as DeckCardLists);

  it('tells a commander deck from a constructed one', () => {
    expect(looksCommander(deck(99, 1))).toBe(true);
    expect(looksCommander(deck(98, 2))).toBe(true);
    expect(looksCommander(deck(60, 15))).toBe(false);
    expect(looksCommander(deck(60, 0))).toBe(false);
    expect(looksCommander(deck(60, 1))).toBe(false);
  });
});

describe('the meta library', () => {
  const library: MetaFormat[] = [
    { id: 'modern', name: 'Modern', snapshot: '2026-09', decks: [{ file: 'modern/burn.dck', name: 'Burn', deckType: 'Constructed - Modern' }] },
    { id: 'edh', name: 'Commander', decks: [{ file: 'edh/atraxa.dck', name: 'Atraxa', deckType: 'Variant Magic - Commander' }] },
    { id: 'empty', name: 'Empty', decks: [] },
  ];
  const commanderType = (deckType?: string) => !!deckType?.includes('Commander');

  it('finds a deck and its format by file', () => {
    expect(findMetaDeck(library, 'edh/atraxa.dck')?.format.id).toBe('edh');
    expect(findMetaDeck(library, 'gone.dck')).toBeNull();
  });

  it('keeps the formats with decks of the kind wanted', () => {
    expect(metaFormatsFor(library, null, commanderType).map((format) => format.id)).toEqual(['modern', 'edh']);
    expect(metaFormatsFor(library, true, commanderType).map((format) => format.id)).toEqual(['edh']);
    expect(metaFormatsFor(library, false, commanderType).map((format) => format.id)).toEqual(['modern']);
  });
});

describe('the inbox', () => {
  it('counts the notes not seen yet', () => {
    expect(unseenCount(undefined)).toBe(0);
    expect(unseenCount([{ id: 1, seen: true }, { id: 2, seen: false }, { id: 3 }])).toBe(2);
  });

  it('says how long ago a note came in the largest whole unit', () => {
    const now = 10_000_000_000;
    expect(agoParts(now - 20_000, now)).toEqual({ value: 0, unit: 'minute' });
    expect(agoParts(now - 5 * 60_000, now)).toEqual({ value: -5, unit: 'minute' });
    expect(agoParts(now - 3 * 3_600_000 - 1, now)).toEqual({ value: -3, unit: 'hour' });
    expect(agoParts(now - 2 * 86_400_000, now)).toEqual({ value: -2, unit: 'day' });
    expect(agoParts(now + 60_000, now)).toEqual({ value: 0, unit: 'minute' });
  });

  it("gives a ghost's record and win share", () => {
    expect(ghostRecord({ won: 3, played: 4 })).toEqual({ won: 3, played: 4, percent: 75 });
    expect(ghostRecord({})).toEqual({ won: 0, played: 0, percent: null });
  });
});

describe('adaptiveSentence', () => {
  const t = ((key: string, vars?: Record<string, unknown>) => `${key}${vars ? JSON.stringify(vars) : ''}`) as Parameters<typeof adaptiveSentence>[0];

  it('bands the skill like the server and never shows it', () => {
    expect([1, 3, 5, 7, 10].map(adaptivePace)).toEqual(['gentle', 'easy', 'fair', 'hard', 'max']);
    expect(adaptiveSentence(t, undefined)).toBeNull();
    expect(adaptiveSentence(t, { skill: 4, recentGames: 1, recentWins: 1 })).toBe('solo.adaptive.sentenceNew{"pace":"solo.adaptive.pace.easy"}');
    const line = adaptiveSentence(t, { skill: 7, recentGames: 10, recentWins: 6 })!;
    expect(line).toContain('"wins":6');
    expect(line).not.toContain('"skill"');
  });
});

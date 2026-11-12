import { describe, expect, test } from 'vitest';
import { cleanTag, cleanTags, filterShelf, MAX_TAG_LENGTH, MAX_TAGS, NO_SHELF_FILTER, parseTags, tagCounts } from './tags';

describe('deck tags (#200)', () => {
  test('a tag is trimmed, its spaces folded, and kept short', () => {
    expect(cleanTag('  Budget   aggro ')).toBe('Budget aggro');
    expect(cleanTag('x'.repeat(40))).toHaveLength(MAX_TAG_LENGTH);
    expect(cleanTag('   ')).toBe('');
  });

  test('repeats go (ignoring case, the first spelling wins), and a deck keeps a few', () => {
    expect(cleanTags(['Aggro', 'aggro', ' AGGRO ', 'Ideas', ''])).toEqual(['Aggro', 'Ideas']);
    expect(cleanTags(Array.from({ length: 10 }, (_, i) => `t${i}`))).toHaveLength(MAX_TAGS);
    expect(cleanTags(undefined)).toEqual([]);
    expect(cleanTags([3 as unknown as string, 'ok'])).toEqual(['ok']);
  });

  test('one field takes several tags, split on commas', () => {
    expect(parseTags('aggro, budget,,  ideas ')).toEqual(['aggro', 'budget', 'ideas']);
  });

  test('every tag in use, counted and sorted', () => {
    expect(tagCounts([{ tags: ['Burn', 'budget'] }, { tags: ['Budget'] }, {}])).toEqual([
      { tag: 'budget', count: 2 },
      { tag: 'Burn', count: 1 },
    ]);
  });

  test('the shelf filters by one tag and can hide drafts', () => {
    const decks = [
      { id: 'a', tags: ['Burn'] },
      { id: 'b', tags: ['burn', 'Ideas'], draft: true },
      { id: 'c' },
    ];
    const ids = (filter: Partial<typeof NO_SHELF_FILTER>) => filterShelf(decks, { ...NO_SHELF_FILTER, ...filter }).map((deck) => deck.id);
    expect(ids({})).toEqual(['a', 'b', 'c']);
    expect(ids({ tag: 'BURN' })).toEqual(['a', 'b']);
    expect(ids({ hideDrafts: true })).toEqual(['a', 'c']);
    expect(ids({ tag: 'ideas', hideDrafts: true })).toEqual([]);
  });
});

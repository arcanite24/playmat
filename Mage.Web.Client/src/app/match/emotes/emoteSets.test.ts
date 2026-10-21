import { describe, expect, test } from 'vitest';
import type { CareerCatalogItem } from '../../../protocol/generated/views';
import { findSticker, ownedEmoteSets, stickerPhrase } from './emoteSets';
import { mutedByChoice } from '../emotes';

const SETS: CareerCatalogItem[] = [
  { kind: 'emote', id: 'phrases', free: true, stickers: [{ id: 'hello', phrase: 'Hello!', es: '¡Hola!' }] },
  { kind: 'emote', id: 'critters', stickers: [{ id: 'owl-sleepy', phrase: 'Zzz…' }] },
  { kind: 'emote', id: 'weather', stickers: [{ id: 'sun', phrase: 'Sunny' }] },
  { kind: 'arrow', id: 'ember', color: '#e0602a' },
];

describe('quick emotes', () => {
  test('the free phrases and the sets this account unlocked, free first', () => {
    const sets = ownedEmoteSets(SETS, [{ kind: 'emote', id: 'weather' }, { kind: 'sleeve', id: 'critters' }]);
    expect(sets.map((set) => set.id)).toEqual(['phrases', 'weather']);
    expect(sets[0].free).toBe(true);
  });

  test('without the catalog, everyone still has the free phrases', () => {
    const sets = ownedEmoteSets([], [{ kind: 'emote', id: 'weather' }]);
    expect(sets.map((set) => set.id)).toEqual(['phrases']);
    expect(sets[0].stickers.map((sticker) => sticker.id)).toContain('good-game');
  });

  test("a sticker is found by id and read in the reader's language", () => {
    expect(findSticker('owl-sleepy', SETS)).toMatchObject({ free: false, sticker: { phrase: 'Zzz…' } });
    expect(findSticker('thanks', [])?.free).toBe(true);
    expect(findSticker('nope', SETS)).toBeNull();
    expect(stickerPhrase(findSticker('hello', SETS)?.sticker, 'es')).toBe('¡Hola!');
    expect(stickerPhrase(findSticker('owl-sleepy', SETS)?.sticker, 'es')).toBe('Zzz…');
  });

  test("muting opponents' emotes hides everyone's but your own", () => {
    expect(mutedByChoice('Bob', true, 'alice')).toBe(true);
    expect(mutedByChoice('Alice', true, 'alice')).toBe(false);
    expect(mutedByChoice('Bob', false, 'alice')).toBe(false);
  });
});

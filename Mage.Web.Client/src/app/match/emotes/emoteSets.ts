import type { CareerCatalogItem, CareerCosmetic, CareerSticker } from '../../../protocol/generated/views';

/**
 * Quick emotes (B298): the sets a player can send from in a match. Everyone has the free table talk (text-only
 * phrases); Career's earned sets add painted stickers. The server checks the same thing (the sticker is in a set the
 * account owns, or in the free one) and sends what was sent to everyone at the table as a `gameEmote` callback.
 */

/** The free set's id in the catalog: phrases only, no pictures. */
export const FREE_SET = 'phrases';

/**
 * The free phrases as the server ships them (Mage.Server career/cosmetics.json), for a client that hasn't got the
 * catalog (it loads with Career): everyone can still say hello.
 */
const FREE_PHRASES: CareerCatalogItem = {
  kind: 'emote',
  id: FREE_SET,
  name: 'Table talk',
  es: 'Charla de mesa',
  free: true,
  stickers: [
    { id: 'hello', phrase: 'Hello!', es: '¡Hola!' },
    { id: 'good-game', phrase: 'Good game', es: 'Buena partida' },
    { id: 'well-played', phrase: 'Well played', es: 'Bien jugado' },
    { id: 'thinking', phrase: 'Thinking…', es: 'Pensando…' },
    { id: 'oops', phrase: 'Oops', es: 'Ups' },
    { id: 'thanks', phrase: 'Thanks', es: 'Gracias' },
  ],
};

export interface EmoteSet {
  id: string;
  item: CareerCatalogItem;
  /** phrases only: drawn as words, never as pictures */
  free: boolean;
  stickers: CareerSticker[];
}

/**
 * The sets this player can send from, free ones first, then the earned ones in catalog order. `sets` are the catalog's
 * emote items; `unlocks` the account's Career unlocks (an emote set unlocks as kind 'emote' with the set's id).
 */
export function ownedEmoteSets(sets: readonly CareerCatalogItem[], unlocks: readonly CareerCosmetic[]): EmoteSet[] {
  const owned = new Set(unlocks.filter((unlock) => unlock.kind === 'emote').map((unlock) => unlock.id));
  const emoteSets = sets.filter((set) => set.kind === 'emote' && set.id && (set.stickers?.length ?? 0) > 0);
  const free = emoteSets.filter((set) => set.free);
  const earned = emoteSets.filter((set) => !set.free && owned.has(set.id));
  const list = free.some((set) => set.id === FREE_SET) ? [...free, ...earned] : [FREE_PHRASES, ...free, ...earned];
  return list.map((item) => ({ id: item.id!, item, free: !!item.free, stickers: (item.stickers ?? []).filter((sticker) => !!sticker.id) }));
}

/** A sticker by id: from the catalog's sets, or the built-in free phrases. */
export function findSticker(id: string | undefined, sets: readonly CareerCatalogItem[]): { sticker: CareerSticker; free: boolean } | null {
  if (!id) return null;
  for (const set of [...sets.filter((one) => one.kind === 'emote'), FREE_PHRASES]) {
    const sticker = set.stickers?.find((one) => one.id === id);
    if (sticker) return { sticker, free: !!set.free };
  }
  return null;
}

/** A sticker's words in the reader's language. */
export function stickerPhrase(sticker: CareerSticker | undefined, locale: string): string {
  if (!sticker) return '';
  return (locale.startsWith('es') ? sticker.es : undefined) ?? sticker.phrase ?? sticker.id ?? '';
}

/** A painted sticker's picture (generated art; it may still be missing, so it is always drawn with its words as a fallback). */
export function stickerArt(id: string): string {
  return `/career-art/sticker/${id}.webp`;
}

/** How long a sent emote locks the wheel: the server takes one every 2.5 seconds. */
export const EMOTE_COOLDOWN_MS = 2500;

import { create } from 'zustand';
import type { CareerCatalogItem } from '../../protocol/generated/views';
import { readJson, writeJson } from '../stores/persist';

/**
 * The cosmetics catalog (B293): every cosmetic progression gives, as the server serves it (careerCosmetics, from
 * Mage.Server resources/career/cosmetics.json), with names, rarity, how each looks and where it comes from. The last
 * catalog is kept in this browser so a match drawn before it loads still knows the sleeves and cloths. The supporters'
 * thank-you set isn't in it (support/thankYou.ts).
 */

export type CatalogKind = 'avatar' | 'title' | 'frame' | 'sleeve' | 'playmat' | 'pet' | 'emote' | 'arrow' | 'spark' | 'stamp' | 'land';
export type Rarity = 'common' | 'uncommon' | 'rare' | 'mythic';

const CACHE_KEY = 'playmat.cosmetics';

interface CatalogState {
  items: CareerCatalogItem[];
  byKey: Map<string, CareerCatalogItem>;
}

const key = (kind: string | undefined, id: string | undefined) => `${kind}:${id}`;

function stateOf(items: CareerCatalogItem[]): CatalogState {
  return { items, byKey: new Map(items.map((item) => [key(item.kind, item.id), item])) };
}

export const useCosmetics = create<CatalogState>(() => stateOf(readJson<CareerCatalogItem[]>(CACHE_KEY, [])));

const listeners = new Set<(items: CareerCatalogItem[]) => void>();

/** Called with the catalog now and whenever a new one arrives (plain modules that keep lookups, like sleeveArt). */
export function onCatalog(listener: (items: CareerCatalogItem[]) => void): void {
  listeners.add(listener);
  listener(useCosmetics.getState().items);
}

export function setCatalog(items: CareerCatalogItem[]): void {
  writeJson(CACHE_KEY, items);
  useCosmetics.setState(stateOf(items));
  listeners.forEach((listener) => listener(items));
}

let loading: Promise<void> | null = null;

/** Fetches the catalog once per page load; failures keep the cached one. */
export function loadCatalog(fetch: () => Promise<CareerCatalogItem[]>): Promise<void> {
  loading ??= fetch().then(setCatalog, () => {
    loading = null;
  });
  return loading;
}

export function catalogItem(kind: string | undefined, id: string | undefined | null): CareerCatalogItem | undefined {
  return id ? useCosmetics.getState().byKey.get(key(kind, id)) : undefined;
}

/** A kind's items in catalog order. */
export function catalogOf(kind: CatalogKind, items: readonly CareerCatalogItem[] = useCosmetics.getState().items): CareerCatalogItem[] {
  return items.filter((item) => item.kind === kind);
}

/** An item's name in the reader's language ("es" or English). */
export function catalogName(item: Pick<CareerCatalogItem, 'name' | 'es' | 'id'> | undefined, locale: string): string {
  if (!item) return '';
  return (locale.startsWith('es') ? item.es : undefined) ?? item.name ?? item.id ?? '';
}

export function rarityOf(item: CareerCatalogItem | undefined): Rarity {
  const rarity = item?.rarity;
  return rarity === 'uncommon' || rarity === 'rare' || rarity === 'mythic' ? rarity : 'common';
}

/** The rarity colours, shared by titles, locker tiles and reward lines. */
export const RARITY_COLORS: Record<Rarity, string> = {
  common: '#c9c3b4',
  uncommon: '#9fc3d6',
  rare: '#e2c27a',
  mythic: '#f08a4b',
};

/** The emote set and sticker of a sticker id. */
export function stickerOf(id: string | undefined): { set: CareerCatalogItem; phrase: string; es?: string; free: boolean } | null {
  if (!id) return null;
  for (const set of catalogOf('emote')) {
    const sticker = set.stickers?.find((one) => one.id === id);
    if (sticker) return { set, phrase: sticker.phrase ?? id, es: sticker.es, free: !!set.free };
  }
  return null;
}

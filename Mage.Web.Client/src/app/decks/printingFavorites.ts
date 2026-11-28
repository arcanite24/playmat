import { create } from 'zustand';
import { readJson, writeJson } from '../stores/persist';

const STORAGE_KEY = 'playmat.printingFavorites';

function loadFavorites(): Set<string> {
  const saved = readJson<unknown>(STORAGE_KEY, []);
  return new Set(Array.isArray(saved) ? saved.filter((item): item is string => typeof item === 'string') : []);
}

interface PrintingFavoritesState {
  /** favorited printing keys (entryKey: "Card Name|SET|123"), pinned to the top of PrintingPicker */
  favorites: ReadonlySet<string>;
  toggle(key: string): void;
  reset(): void;
}

export const usePrintingFavorites = create<PrintingFavoritesState>((set) => ({
  favorites: loadFavorites(),
  toggle: (key) => set((state) => {
    const next = new Set(state.favorites);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    writeJson(STORAGE_KEY, [...next]);
    return { favorites: next };
  }),
  reset: () => {
    writeJson(STORAGE_KEY, []);
    set({ favorites: new Set() });
  },
}));

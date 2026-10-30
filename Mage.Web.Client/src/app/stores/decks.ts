import { create } from 'zustand';
import { formatRules, isCommanderFormat } from '../../core/decks/formats';
import { deckStorage, type DeckSummary } from '../../core/decks/DeckStorageService';
import { DeckSerializer } from '../../core/decks/DeckSerializer';
import { copyName } from '../../core/decks/exportFormats';
import type { DeckCardLists, DeckImportSource } from '../../core/decks/types';
import { pickMetaDeck, type MetaDeck, type MetaDeckIndex } from '../../core/decks/metaDecks';
import { readJson, writeJson } from './persist';
import { RANDOM_SLEEVE, randomSleeveFor } from '../ui/sleeveArt';

export interface StarterDeck {
  file: string;
  name: string;
  cards: number;
  cover: { name: string; setCode: string; cardNumber: string };
  /** the deck type it is built for; Freeform when absent */
  format?: string;
}

/** A deck in the roster: one the player saved, or a starter deck not saved yet. */
export interface RosterDeck {
  id: string;
  name: string;
  note: string;
  cardCount: number;
  cover: { name?: string; setCode: string; cardNumber: string } | null;
  colors: string[];
  starter: StarterDeck | null;
  /** a saved copy of a starter deck (it keeps the starter's name), made when the starter was first played */
  fromStarter?: boolean;
  /** the deck website it was imported from */
  source?: DeckImportSource;
}

const SELECTED_KEY = 'playmat.selectedDeck';
const SLEEVES_KEY = 'playmat.sleeves';
export const STARTER_PREFIX = 'starter:';
/** an AI format deck (public/meta-decks) in the AI settings */
export const META_PREFIX = 'meta:';
/** a game against the AI with a random format deck for the player ("meta-random:Constructed - Modern"), drawn each game */
export const RANDOM_META_PREFIX = 'meta-random:';

const COLOR_ORDER = ['white', 'blue', 'black', 'red', 'green'] as const;
const COLOR_LETTER: Record<(typeof COLOR_ORDER)[number], string> = { white: 'W', blue: 'U', black: 'B', red: 'R', green: 'G' };

/** Sleeve colors: matte dyes, picked to stay readable around card art. */
export const SLEEVE_COLORS = ['#20302b', '#1f2a44', '#3b1f2b', '#4a2a17', '#2c2c2e', '#3e3a2a', '#24403f', '#5a2320'];

function summaryToRoster(deck: DeckSummary): RosterDeck {
  const colors = COLOR_ORDER.filter((color) => deck.colors?.[color]).map((color) => COLOR_LETTER[color]);
  const size = `${deck.cardCount ?? 0} cards${deck.sideboardCount ? ` + ${deck.sideboardCount}` : ''}`;
  return {
    id: deck.id,
    name: deck.name || 'Untitled deck',
    note: deck.format ? `${formatRules(deck.format).label} · ${size}` : size,
    cardCount: deck.cardCount ?? 0,
    cover: deck.coverCard ?? null,
    colors,
    starter: null,
    source: deck.source,
  };
}

function starterToRoster(starter: StarterDeck): RosterDeck {
  return {
    id: STARTER_PREFIX + starter.file,
    name: starter.name,
    note: `${isCommanderFormat(starter.format) ? 'Commander starter' : 'Starter deck'} · ${starter.cards} cards`,
    cardCount: starter.cards,
    cover: starter.cover,
    colors: [],
    starter,
  };
}

export interface DeckLibraryState {
  saved: RosterDeck[];
  starters: StarterDeck[];
  /** the AI's format decks */
  metaDecks: MetaDeck[];
  selectedId: string | null;
  sleeves: Record<string, string>;
  loaded: boolean;
  refresh(): Promise<void>;
  select(id: string): void;
  setSleeve(id: string, color: string): void;
  /** the deck list to send to the server; starter decks are saved to the library on first use */
  loadForPlay(id: string): Promise<{ deck: DeckCardLists; id: string; meta?: MetaDeck }>;
  /** a deck from the import sheet, saved as a new deck and selected */
  saveImported(deck: DeckCardLists): Promise<string>;
  /** a deck brought up to date with its site, under its own id */
  saveUpdated(deck: DeckCardLists): Promise<void>;
  remove(id: string): Promise<void>;
  /** a saved copy of a deck under a new name ("Burn (copy)"), with the same sleeve; selected, returns its id */
  duplicate(id: string): Promise<string>;
  rename(id: string, name: string): Promise<void>;
  /** the full deck list of a roster deck (saved or starter) without saving anything */
  loadList(id: string): Promise<DeckCardLists>;
}

async function fetchStarters(): Promise<StarterDeck[]> {
  try {
    const response = await fetch('/starter-decks/index.json');
    return response.ok ? ((await response.json()) as StarterDeck[]) : [];
  } catch {
    return [];
  }
}

async function fetchMetaDecks(): Promise<MetaDeck[]> {
  try {
    const response = await fetch('/meta-decks/index.json');
    return response.ok ? ((await response.json()) as Partial<MetaDeckIndex>).decks ?? [] : [];
  } catch {
    return [];
  }
}

/** A shipped deck's list: a starter (public/starter-decks) or a format deck (public/meta-decks). */
async function shippedDeck(folder: string, deck: StarterDeck | MetaDeck): Promise<DeckCardLists> {
  const response = await fetch(`/${folder}/${encodeURIComponent(deck.file)}`);
  const text = response.ok ? await response.text() : '';
  if (!text || /^\s*</.test(text)) throw new Error(`Couldn't load ${deck.name}.`);
  return { ...DeckSerializer.importDeck(text), name: deck.name, coverCard: deck.cover, format: deck.format ?? 'Constructed - Freeform' };
}

/** The deck type a starter deck is built for. */
function starterFormat(starters: StarterDeck[], id: string): string {
  return starters.find((starter) => STARTER_PREFIX + starter.file === id)?.format ?? 'Constructed - Freeform';
}

export const useDecks = create<DeckLibraryState>((set, get) => ({
  saved: [],
  starters: [],
  metaDecks: [],
  selectedId: readJson<string | null>(SELECTED_KEY, null),
  sleeves: readJson<Record<string, string>>(SLEEVES_KEY, {}),
  loaded: false,

  async refresh() {
    const [summaries, starters, metaDecks] = await Promise.all([
      deckStorage.listDecks(),
      get().starters.length ? get().starters : fetchStarters(),
      get().metaDecks.length ? get().metaDecks : fetchMetaDecks(),
    ]);
    const saved = summaries.map(summaryToRoster);
    let selectedId = get().selectedId;
    const known = new Set([...saved.map((deck) => deck.id), ...starters.map((starter) => STARTER_PREFIX + starter.file)]);
    if (!selectedId || !known.has(selectedId)) {
      selectedId = saved[0]?.id ?? (starters[0] ? STARTER_PREFIX + starters[0].file : null);
    }
    set({ saved, starters, metaDecks, selectedId, loaded: true });
  },

  select(id) {
    writeJson(SELECTED_KEY, id);
    set({ selectedId: id });
  },

  setSleeve(id, color) {
    const sleeves = { ...get().sleeves, [id]: color };
    writeJson(SLEEVES_KEY, sleeves);
    set({ sleeves });
  },

  async loadForPlay(id) {
    // a format deck plays as it is, without a copy in the player's library
    if (id.startsWith(META_PREFIX) || id.startsWith(RANDOM_META_PREFIX)) {
      const { metaDecks } = get().metaDecks.length ? get() : (await get().refresh(), get());
      const meta = id.startsWith(META_PREFIX)
        ? metaDecks.find((deck) => META_PREFIX + deck.file === id)
        : pickMetaDeck(metaDecks, id.slice(RANDOM_META_PREFIX.length));
      if (!meta) throw new Error('No format deck is available for that format.');
      return { deck: await shippedDeck('meta-decks', meta), id, meta };
    }
    if (id.startsWith(STARTER_PREFIX)) {
      const deck = await get().loadList(id);
      deck.format = starterFormat(get().starters, id);
      const savedId = await deckStorage.saveDeck(deck, { forceNew: true });
      const sleeve = get().sleeves[id];
      await get().refresh();
      if (sleeve) get().setSleeve(savedId, sleeve);
      get().select(savedId);
      return { deck, id: savedId };
    }
    const deck = await deckStorage.loadDeck(id);
    if (!deck) throw new Error('That deck is no longer saved.');
    return { deck, id };
  },

  async saveImported(deck) {
    const id = await deckStorage.saveDeck({ ...deck, id: undefined }, { forceNew: true });
    await get().refresh();
    get().select(id);
    return id;
  },

  async saveUpdated(deck) {
    await deckStorage.saveDeck(deck);
    await get().refresh();
  },

  async remove(id) {
    await deckStorage.deleteDeck(id);
    await get().refresh();
  },

  async duplicate(id) {
    const deck = await get().loadList(id);
    const roster = rosterOf(get());
    const taken = roster.map((entry) => entry.name);
    // the copy wears the sleeve the original shows, even when that is the original's default
    const sleeve = chosenSleeve(get().sleeves, roster.find((entry) => entry.id === id));
    const starter = id.startsWith(STARTER_PREFIX);
    const name = copyName(deck.name, taken);
    // a copy is the player's own deck: it no longer follows the site it was imported from
    const copy: DeckCardLists = { ...deck, id: undefined, name, source: undefined, createdAt: undefined };
    if (starter) copy.format = starterFormat(get().starters, id);
    const newId = await deckStorage.saveDeck(copy, { forceNew: true });
    await get().refresh();
    get().setSleeve(newId, sleeve);
    get().select(newId);
    return newId;
  },

  async rename(id, name) {
    const trimmed = name.trim();
    if (!trimmed) throw new Error('A deck needs a name.');
    const deck = await deckStorage.loadDeck(id);
    if (!deck) throw new Error('That deck is no longer saved.');
    await deckStorage.saveDeck({ ...deck, id, name: trimmed });
    await get().refresh();
  },

  async loadList(id) {
    if (id.startsWith(STARTER_PREFIX)) {
      const starter = get().starters.find((candidate) => STARTER_PREFIX + candidate.file === id);
      if (!starter) throw new Error('That starter deck is not available.');
      return shippedDeck('starter-decks', starter);
    }
    const deck = await deckStorage.loadDeck(id);
    if (!deck) throw new Error('That deck is no longer saved.');
    return { ...deck, id };
  },
}));

export function rosterOf(state: Pick<DeckLibraryState, 'saved' | 'starters'>, hideStarterDecks = false): RosterDeck[] {
  const savedNames = new Set(state.saved.map((deck) => deck.name));
  const starterNames = new Set(state.starters.map((starter) => starter.name));
  // starters already saved by the player don't show twice: the copy stands in for them, and says where it came from
  const saved = state.saved.map((deck) => (starterNames.has(deck.name) ? { ...deck, fromStarter: true } : deck));
  if (hideStarterDecks) return saved;
  return [...saved, ...state.starters.filter((starter) => !savedNames.has(starter.name)).map(starterToRoster)];
}

export function sleeveFor(sleeves: Record<string, string>, deck: RosterDeck | null | undefined): string {
  if (!deck) return SLEEVE_COLORS[0];
  // "random": one of the player's Career sleeves, a different one each game (ui/sleeveArt.ts)
  if (sleeves[deck.id] === RANDOM_SLEEVE) return randomSleeveFor(deck.id, SLEEVE_COLORS);
  if (sleeves[deck.id]) return sleeves[deck.id];
  // a stable default per deck
  let hash = 0;
  for (const char of deck.name) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return SLEEVE_COLORS[hash % SLEEVE_COLORS.length];
}

/** The sleeve chosen for a deck as the sleeve pickers show it: "random" stays random, anything else is its colour. */
export function chosenSleeve(sleeves: Record<string, string>, deck: RosterDeck | null | undefined): string {
  return deck && sleeves[deck.id] === RANDOM_SLEEVE ? RANDOM_SLEEVE : sleeveFor(sleeves, deck);
}

import type { DeckCardLists, DeckImportSource } from "./types.js";
import { DeckSerializer } from "./DeckSerializer.js";
import { newId } from "../util/uuid.js";

export interface DeckSummary {
    id: string;
    name: string;
    description?: string;
    format?: string;
    updatedAt: number;
    cardCount?: number;
    sideboardCount?: number;
    coverCard?: {
        setCode: string;
        cardNumber: string;
        name?: string;
    };
    colors?: {
        white: boolean;
        blue: boolean;
        black: boolean;
        red: boolean;
        green: boolean;
    };
    /** the deck website the deck was imported from */
    source?: DeckImportSource;
    /** the deck's own token pictures (ui/tokenArt.ts) */
    tokenArt?: Record<string, string>;
    /** the player's labels (#200) */
    tags?: string[];
    /** still being worked on (#200) */
    draft?: boolean;
}

export interface IDeckStorage {
    saveDeck(deck: DeckCardLists, options?: SaveDeckOptions): Promise<string>; // Returns ID
    loadDeck(id: string): Promise<DeckCardLists | null>;
    listDecks(): Promise<DeckSummary[]>;
    deleteDeck(id: string): Promise<void>;
}

export interface SaveDeckOptions {
    forceNew?: boolean;
}

const STORAGE_PREFIX = "mage_deck_";
const META_KEY = "mage_decks_meta";
/** decks deleted in this browser that the server may not know about yet (deck sync) */
const TOMBSTONES_KEY = "playmat.deckTombstones";

/** A change made by the player, for deck sync to pass on. */
export type DeckChange = { type: "saved"; id: string } | { type: "deleted"; id: string; at: number };

interface DeckMeta {
    id: string;
    name: string;
    description?: string;
    format?: string;
    updatedAt: number;
    cardCount?: number;
    sideboardCount?: number;
    coverCard?: {
        setCode: string;
        cardNumber: string;
        name?: string;
    };
    colors?: {
        white: boolean;
        blue: boolean;
        black: boolean;
        red: boolean;
        green: boolean;
    };
    /** the deck website the deck was imported from */
    source?: DeckImportSource;
    /** the deck's own token pictures (ui/tokenArt.ts) */
    tokenArt?: Record<string, string>;
    /** the player's labels (#200) */
    tags?: string[];
    /** still being worked on (#200) */
    draft?: boolean;
}

export class LocalDeckStorage implements IDeckStorage {
    private listeners = new Set<(change: DeckChange) => void>();

    /** Changes made through saveDeck and deleteDeck (not the sync's own writes). */
    onChange(listener: (change: DeckChange) => void): () => void {
        this.listeners.add(listener);
        return () => this.listeners.delete(listener);
    }

    private emit(change: DeckChange) {
        for (const listener of this.listeners) listener(change);
    }

    async saveDeck(deck: DeckCardLists, options: SaveDeckOptions = {}): Promise<string> {
        const meta = this.getMeta();

        // Use existing deck ID if available, otherwise generate new one or find by name
        let id: string;
        if (options.forceNew) {
            id = newId();
        } else if (deck.id) {
            id = deck.id;
        } else {
            const existing = meta.find(m => m.name === deck.name);
            id = existing?.id || newId();
        }

        // Store extended deck data as JSON (preserving metadata)
        const now = Date.now();
        const deckData = {
            ...deck,
            id,
            updatedAt: now,
            createdAt: deck.createdAt || now,
        };
        this.store(deckData, meta);
        this.forgetTombstone(id);
        this.emit({ type: "saved", id });
        return id;
    }

    private store(deck: DeckCardLists & { id: string; updatedAt: number }, meta: DeckMeta[]) {
        localStorage.setItem(STORAGE_PREFIX + deck.id, JSON.stringify(deck));
        const newMetaItem: DeckMeta = {
            id: deck.id,
            name: deck.name || "Untitled Deck",
            description: deck.description,
            format: deck.format,
            updatedAt: deck.updatedAt,
            cardCount: deck.cards.reduce((sum, c) => sum + c.amount, 0),
            sideboardCount: deck.sideboard.reduce((sum, c) => sum + c.amount, 0),
            coverCard: deck.coverCard,
            colors: deck.colors,
            source: deck.source,
            tokenArt: deck.tokenArt,
            tags: deck.tags,
            draft: deck.draft,
        };
        const newMeta = meta.filter(m => m.id !== deck.id);
        newMeta.push(newMetaItem);
        this.saveMeta(newMeta);
    }

    /** Every saved deck's id and last change, for deck sync. */
    stamps(): { id: string; updatedAt: number }[] {
        return this.getMeta().map(m => ({ id: m.id, updatedAt: m.updatedAt }));
    }

    /** The deck as JSON for the server, with its own change time. */
    async exportForSync(id: string): Promise<{ name: string; updatedAt: number; data: string } | null> {
        const deck = await this.loadDeck(id);
        const meta = this.getMeta().find(m => m.id === id);
        if (!deck || !meta) return null;
        const data = { ...deck, id, updatedAt: meta.updatedAt };
        return { name: meta.name, updatedAt: meta.updatedAt, data: JSON.stringify(data) };
    }

    /** A deck from the server, kept as it is (its change time too); not reported as a change. */
    importFromSync(id: string, data: string): boolean {
        let deck: DeckCardLists;
        try {
            deck = JSON.parse(data);
        } catch {
            return false;
        }
        if (!deck || !Array.isArray(deck.cards)) return false;
        const updatedAt = typeof deck.updatedAt === "number" ? deck.updatedAt : Date.now();
        this.store({ ...deck, sideboard: deck.sideboard ?? [], id, updatedAt }, this.getMeta());
        this.forgetTombstone(id);
        return true;
    }

    /** Deleted on another device: gone here too, without a tombstone. */
    removeFromSync(id: string) {
        localStorage.removeItem(STORAGE_PREFIX + id);
        this.saveMeta(this.getMeta().filter(m => m.id !== id));
    }

    tombstones(): Record<string, number> {
        try {
            return JSON.parse(localStorage.getItem(TOMBSTONES_KEY) ?? "{}") ?? {};
        } catch {
            return {};
        }
    }

    forgetTombstone(id: string) {
        const tombstones = this.tombstones();
        if (!(id in tombstones)) return;
        delete tombstones[id];
        localStorage.setItem(TOMBSTONES_KEY, JSON.stringify(tombstones));
    }

    async loadDeck(id: string): Promise<DeckCardLists | null> {
        const content = localStorage.getItem(STORAGE_PREFIX + id);
        if (!content) return null;

        // Try parsing as JSON first (new format), fall back to .dck format
        try {
            const parsed = JSON.parse(content);
            // Check if it looks like a deck object (has cards array)
            if (parsed && Array.isArray(parsed.cards)) {
                return parsed as DeckCardLists;
            }
        } catch {
            // Not JSON, try legacy .dck format
        }

        // Legacy format: parse with DeckSerializer
        const deck = DeckSerializer.importDeck(content);
        const meta = this.getMeta();
        const m = meta.find(x => x.id === id);
        if (m) {
            deck.name = m.name;
            deck.description = m.description;
            deck.coverCard = m.coverCard;
            deck.colors = m.colors;
        }
        deck.id = id;
        return deck;
    }

    async listDecks(): Promise<DeckSummary[]> {
        const meta = this.getMeta();
        return meta.map(m => ({
            id: m.id,
            name: m.name,
            description: m.description,
            format: m.format,
            updatedAt: m.updatedAt,
            cardCount: m.cardCount,
            sideboardCount: m.sideboardCount,
            coverCard: m.coverCard,
            colors: m.colors,
            source: m.source,
            tokenArt: m.tokenArt,
            tags: m.tags,
            draft: m.draft,
        })).sort((a, b) => b.updatedAt - a.updatedAt);
    }

    async deleteDeck(id: string): Promise<void> {
        localStorage.removeItem(STORAGE_PREFIX + id);
        const meta = this.getMeta();
        this.saveMeta(meta.filter(m => m.id !== id));
        const at = Date.now();
        localStorage.setItem(TOMBSTONES_KEY, JSON.stringify({ ...this.tombstones(), [id]: at }));
        this.emit({ type: "deleted", id, at });
    }

    private getMeta(): DeckMeta[] {
        const s = localStorage.getItem(META_KEY);
        if (!s) return [];
        try {
            return JSON.parse(s);
        } catch {
            return [];
        }
    }

    private saveMeta(meta: DeckMeta[]) {
        localStorage.setItem(META_KEY, JSON.stringify(meta));
    }
}

export const deckStorage = new LocalDeckStorage();

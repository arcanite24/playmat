import {
  exceptionLink,
  isTokenImage,
  XMAGE_TOKENS_SET_CODE,
  namedImageLink,
  printingKey,
  setCodeOf,
  normalizeCollectorNumber,
  toImageLink,
  type CardFace,
  type CardImageRef,
  type ImageSize,
  type ScryfallLinks,
} from './imageLinks';
import { configuredImageProxy, proxiedScryfallUrl } from './imageProxy';

type ImageUris = Partial<Record<ImageSize, string>>;

/** Image links of one printing, as returned by Scryfall. */
export interface PrintingImages {
  front?: ImageUris;
  back?: ImageUris;
  notFound?: boolean;
  fetchedAt: number;
}

/** Persistent cache of printing links (IndexedDB in browsers, memory in tests). */
export interface ImageLinkStore {
  get(key: string): Promise<PrintingImages | undefined>;
  setMany(entries: [string, PrintingImages][]): Promise<void>;
  /** everything stored, read once at start so known printings resolve on their first frame */
  getAll?(): Promise<[string, PrintingImages][]>;
}

export interface ImageResolverOptions {
  fetch?: typeof fetch;
  store?: ImageLinkStore;
  linksUrl?: string;
  /** Scryfall asks clients to stay under 10 requests per second */
  minRequestIntervalMs?: number;
  batchDelayMs?: number;
  /** first wait before retrying a lookup that was refused or failed; doubled each time */
  retryDelayMs?: number;
  /** cached links older than this are refreshed */
  maxAgeMs?: number;
  now?: () => number;
  /**
   * Prefix of the caching proxy for Scryfall (e.g. "/img"); defaults to the build's VITE_IMAGE_PROXY. Empty or
   * undefined talks to Scryfall directly. Applied when a link is used, never to the stored links.
   */
  imageProxy?: string;
}

interface ScryfallCard {
  set?: string;
  collector_number?: string;
  image_uris?: ImageUris;
  card_faces?: { image_uris?: ImageUris }[];
}

const MAX_BATCH = 75; // Scryfall /cards/collection limit
const COLLECTION_URL = 'https://api.scryfall.com/cards/collection';
const DAY = 24 * 60 * 60 * 1000;

/**
 * Resolves card pictures to Scryfall CDN links (through the deployment's caching proxy when the build has one).
 *
 * Printings are looked up in batches of up to 75 with POST /cards/collection, rate limited, and the links are
 * cached persistently. The browser's HTTP cache then keeps the image files themselves, so nothing is
 * downloaded twice and a full board costs one or two API requests instead of one per card.
 */
export class ImageResolver {
  private readonly options: Required<Omit<ImageResolverOptions, 'store' | 'imageProxy'>> & {
    store: ImageLinkStore | null;
    imageProxy: string | undefined;
  };
  private readonly memory = new Map<string, PrintingImages>();
  private readonly inFlight = new Set<string>();
  private readonly queue: { key: string; set: string; number: string }[] = [];
  private readonly listeners = new Set<() => void>();
  private readonly keyListeners = new Map<string, Set<() => void>>();
  private readonly linkListeners = new Set<() => void>();
  private links: ScryfallLinks | null = null;
  private linksPromise: Promise<void> | null = null;
  private batchTimer: ReturnType<typeof setTimeout> | null = null;
  private draining = false;
  private lastRequestAt = 0;
  private version = 0;

  constructor(options: ImageResolverOptions = {}) {
    this.options = {
      fetch: options.fetch ?? ((...args) => fetch(...args)),
      store: options.store ?? null,
      linksUrl: options.linksUrl ?? '/data/scryfall-links.json',
      minRequestIntervalMs: options.minRequestIntervalMs ?? 120,
      batchDelayMs: options.batchDelayMs ?? 40,
      retryDelayMs: options.retryDelayMs ?? 1000,
      maxAgeMs: options.maxAgeMs ?? 30 * DAY,
      now: options.now ?? (() => Date.now()),
      imageProxy: 'imageProxy' in options ? options.imageProxy : configuredImageProxy(),
    };
    this.hydrate();
  }

  /**
   * Reads every stored link into memory once. Until then each printing is read from the store on its own, which takes
   * a turn of the event loop: long enough for a card to show its text frame before its picture.
   */
  private hydrate(): void {
    const all = this.options.store?.getAll?.();
    if (!all) return;
    all.then((entries) => {
      const keys: string[] = [];
      for (const [key, entry] of entries) {
        if (this.memory.has(key) || this.isStale(entry)) continue;
        this.memory.set(key, entry);
        keys.push(key);
      }
      if (keys.length > 0) this.notify(keys);
    }, () => undefined);
  }

  /** Changes whenever new links arrive (for useSyncExternalStore). */
  getVersion(): number {
    return this.version;
  }

  /** Called after every change (any printing). Prefer {@link subscribeCard} for a single card. */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Called only when this card's picture may have changed: its printing's links arrived (or failed), or the
   * exported link lists loaded. Other cards resolving do not call it, so a board of cards does not re-render
   * every card for every picture.
   */
  subscribeCard(card: CardImageRef | null | undefined, listener: () => void): () => void {
    return this.subscribeKey(card && !isTokenImage(card) ? printingKey(card) : null, listener);
  }

  /** Per-printing subscription (`set/number`); a null key only hears about the exported link lists loading. */
  subscribeKey(key: string | null, listener: () => void): () => void {
    if (key === null) {
      this.linkListeners.add(listener);
      return () => this.linkListeners.delete(listener);
    }
    let set = this.keyListeners.get(key);
    if (!set) {
      set = new Set();
      this.keyListeners.set(key, set);
    }
    set.add(listener);
    return () => {
      const current = this.keyListeners.get(key);
      if (!current) return;
      current.delete(listener);
      if (current.size === 0) this.keyListeners.delete(key);
    };
  }

  /**
   * The best known image link, or null while it is being looked up (call again after the next notification).
   * Returns undefined when the card has no known picture at all, so the UI can draw a text frame instead.
   */
  resolve(card: CardImageRef, face: CardFace = 'front', size: ImageSize = 'normal'): string | null | undefined {
    const link = this.resolveDirect(card, face, size);
    return typeof link === 'string' ? proxiedScryfallUrl(link, this.options.imageProxy) : link;
  }

  /** {@link resolve} before the proxy rewrite: Scryfall's own links. */
  private resolveDirect(card: CardImageRef, face: CardFace, size: ImageSize): string | null | undefined {
    this.ensureLinks();
    const exception = exceptionLink(card, face, this.links);
    if (exception) return toImageLink(exception, size);
    if (isTokenImage(card)) {
      // tokens without a mapped picture have no reliable Scryfall printing
      return this.links ? undefined : null;
    }

    const key = printingKey(card);
    if (!key) {
      // command objects without a collector number may still be in the token lists
      if (!this.links) return null;
      return card.name ? namedImageLink(card, face, size) : undefined;
    }

    const cached = this.memory.get(key);
    if (cached && !this.isStale(cached)) {
      if (cached.notFound) return namedImageLink(card, face, size);
      const uris = face === 'back' ? cached.back ?? cached.front : cached.front;
      return uris?.[size] ?? uris?.normal ?? uris?.large ?? namedImageLink(card, face, size);
    }

    this.request(key, setCodeOf(card), normalizeCollectorNumber(card.cardNumber));
    return null;
  }

  /**
   * The token lists' pictures (token art choices, #226): with a name, that token's pictures as the lists key them
   * ("SET/Name" or "SET/Name/number"); without, every token name. Null until the lists have loaded (subscribeKey(null)
   * hears when they do). XMage's own pictures (face down, copy...) are left out.
   */
  /** Whether the exported link lists have loaded (asks for them when not). */
  linksLoaded(): boolean {
    this.ensureLinks();
    return this.links !== null;
  }

  tokenPictures(name?: string): string[] | null {
    this.ensureLinks();
    if (!this.links) return null;
    const keys = Object.keys(this.links.tokens).filter((key) => !key.toUpperCase().startsWith(`${XMAGE_TOKENS_SET_CODE}/`));
    if (name === undefined) return [...new Set(keys.map((key) => key.split('/')[1]))].sort((a, b) => a.localeCompare(b));
    return keys.filter((key) => key.split('/')[1] === name).sort();
  }

  /** Look up many cards ahead of time (deck lists, a new board). */
  prefetch(cards: Iterable<CardImageRef>): void {
    this.ensureLinks();
    for (const card of cards) {
      if (isTokenImage(card)) continue;
      const key = printingKey(card);
      if (key && !this.memory.has(key)) this.request(key, setCodeOf(card), normalizeCollectorNumber(card.cardNumber));
    }
  }

  private isStale(entry: PrintingImages): boolean {
    return this.options.now() - entry.fetchedAt > this.options.maxAgeMs;
  }

  private ensureLinks(): void {
    if (this.links || this.linksPromise) return;
    this.linksPromise = this.options.fetch(this.options.linksUrl)
      .then((response) => (response.ok ? response.json() : null))
      .then((links: ScryfallLinks | null) => {
        this.links = links ?? { cards: {}, tokens: {} };
        this.notify(null);
      })
      .catch(() => {
        this.links = { cards: {}, tokens: {} };
        this.notify(null);
      });
  }

  private request(key: string, set: string, number: string): void {
    if (this.inFlight.has(key)) return;
    this.inFlight.add(key);
    const store = this.options.store;
    const enqueue = () => {
      this.queue.push({ key, set, number });
      this.scheduleBatch();
    };
    if (!store) {
      enqueue();
      return;
    }
    store.get(key).then((stored) => {
      if (stored && !this.isStale(stored)) {
        this.memory.set(key, stored);
        this.inFlight.delete(key);
        this.notify([key]);
      } else {
        enqueue();
      }
    }, enqueue);
  }

  private scheduleBatch(): void {
    if (this.batchTimer || this.draining) return;
    this.batchTimer = setTimeout(() => {
      this.batchTimer = null;
      void this.drain();
    }, this.options.batchDelayMs);
  }

  private async drain(): Promise<void> {
    if (this.draining) return;
    this.draining = true;
    try {
      while (this.queue.length > 0) {
        const batch = this.queue.splice(0, MAX_BATCH);
        const wait = this.lastRequestAt + this.options.minRequestIntervalMs - this.options.now();
        if (wait > 0) await new Promise((resolve) => setTimeout(resolve, wait));
        this.lastRequestAt = this.options.now();
        await this.fetchBatch(batch);
      }
    } finally {
      this.draining = false;
      if (this.queue.length > 0) this.scheduleBatch();
    }
  }

  private async fetchBatch(batch: { key: string; set: string; number: string }[], attempt = 0): Promise<void> {
    let response: Response;
    try {
      response = await this.options.fetch(proxiedScryfallUrl(COLLECTION_URL, this.options.imageProxy), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify({
          identifiers: batch.map((item) => ({ set: item.set.toLowerCase(), collector_number: item.number })),
        }),
      });
    } catch {
      response = Response.error();
    }

    // rate limited, a dropped connection or a passing proxy error: try again before giving up, because giving up
    // sends every card in the batch to Scryfall on its own, which hits Scryfall's limit and loses pictures
    const transient = response.status === 0 || response.status === 429 || response.status >= 500;
    if (transient && attempt < 3) {
      await new Promise((resolve) => setTimeout(resolve, this.options.retryDelayMs * 2 ** attempt));
      await this.fetchBatch(batch, attempt + 1);
      return;
    }
    if (!response.ok) {
      this.giveUp(batch);
      return;
    }

    const body = await response.json() as { data?: ScryfallCard[] };
    const now = this.options.now();
    const found = new Map<string, PrintingImages>();
    for (const card of body.data ?? []) {
      const key = `${(card.set ?? '').toLowerCase()}/${(card.collector_number ?? '').toLowerCase()}`;
      const faces = card.card_faces ?? [];
      found.set(key, {
        front: card.image_uris ?? faces[0]?.image_uris,
        back: card.image_uris ? undefined : faces[1]?.image_uris,
        fetchedAt: now,
      });
    }

    const entries: [string, PrintingImages][] = batch.map((item) => [
      item.key,
      found.get(item.key) ?? { notFound: true, fetchedAt: now },
    ]);
    for (const [key, entry] of entries) {
      this.memory.set(key, entry);
      this.inFlight.delete(key);
    }
    this.options.store?.setMany(entries).catch(() => undefined);
    this.notify(batch.map((item) => item.key));
  }

  /** Network trouble: fall back to direct links for this session, retry on the next visit. */
  private giveUp(batch: { key: string }[]): void {
    for (const item of batch) {
      this.memory.set(item.key, { notFound: true, fetchedAt: this.options.now() - this.options.maxAgeMs + 60_000 });
      this.inFlight.delete(item.key);
    }
    this.notify(batch.map((item) => item.key));
  }

  /** keys = the printings that changed; null = the link lists loaded, which can change any card. */
  private notify(keys: readonly string[] | null): void {
    this.version++;
    const called = new Set<() => void>();
    const call = (listener: () => void) => {
      if (called.has(listener)) return;
      called.add(listener);
      listener();
    };
    if (keys === null) {
      [...this.linkListeners].forEach(call);
      for (const set of [...this.keyListeners.values()]) [...set].forEach(call);
    } else {
      for (const key of keys) [...(this.keyListeners.get(key) ?? [])].forEach(call);
    }
    [...this.listeners].forEach(call);
  }
}

import { imageResolver } from '../../core/images/useCardImage';
import type { CardImageRef, ImageSize } from '../../core/images/imageLinks';
import { DEVICE_PICTURES, storePictures } from '../nativePictures';

/**
 * Card pictures already downloaded and decoded in this session. A card face whose picture is here shows it on its
 * first frame instead of flashing the text frame while the browser reports the load again.
 */
const decoded = new Set<string>();
const pending = new Map<string, Promise<void>>();
/** keep decoded bitmaps referenced so the browser keeps them warm; two 75-card decks at three sizes fit */
const held: HTMLImageElement[] = [];
export const MAX_HELD = 1200;

/**
 * Warming waits its turn: at most this many pictures download at once, at low fetch priority, so a whole deck being
 * warmed never holds up the pictures the screen is showing right now (those load through their own <img>).
 */
export const MAX_WARMING = 4;
export type WarmPriority = 'high' | 'low';
interface Waiting {
  src: string;
  priority: WarmPriority;
  start: () => void;
}
const waiting: Waiting[] = [];
let active = 0;

/** pictures already asked of the browser's cache, so a card re-rendering doesn't ask again */
const probed = new Set<string>();

/**
 * Whether a picture can show on the first frame: decoded earlier this session, or ready in the browser's image cache
 * (an earlier visit, another tab). An image the browser has in memory reports `complete` as soon as its src is set;
 * one it doesn't is let go at once, so probing never downloads a picture the card face would load lazily.
 */
export function isDecoded(src: string): boolean {
  if (decoded.has(src)) return true;
  if (probed.has(src) || typeof Image === 'undefined') return false;
  probed.add(src);
  const image = new Image();
  image.src = src;
  if (image.complete && image.naturalWidth > 0) {
    decoded.add(src);
    hold(image);
    return true;
  }
  image.src = '';
  return false;
}

function hold(image: HTMLImageElement) {
  held.push(image);
  if (held.length > MAX_HELD) held.shift();
}

export function markDecoded(src: string) {
  decoded.add(src);
}

/**
 * Download and decode a picture ahead of time. High priority (cards in play) goes ahead of low priority (decks warmed
 * before a match), which only downloads while the browser has nothing better to do.
 */
export function warmImage(src: string, priority: WarmPriority = 'high'): Promise<void> {
  if (decoded.has(src)) return Promise.resolve();
  const existing = pending.get(src);
  if (existing) {
    if (priority === 'high') promote(src);
    return existing;
  }
  const done = new Promise<void>((resolve) => {
    const entry: Waiting = { src, priority, start: () => void load(src, priority).finally(resolve) };
    if (priority === 'high') waiting.splice(firstLow(), 0, entry);
    else waiting.push(entry);
  }).finally(() => pending.delete(src));
  pending.set(src, done);
  pump();
  return done;
}

function firstLow(): number {
  const index = waiting.findIndex((entry) => entry.priority === 'low');
  return index < 0 ? waiting.length : index;
}

/** A queued low-priority picture that the board now wants moves up with the other high-priority ones. */
function promote(src: string) {
  const index = waiting.findIndex((entry) => entry.src === src && entry.priority === 'low');
  if (index < 0) return;
  const [entry] = waiting.splice(index, 1);
  entry.priority = 'high';
  waiting.splice(firstLow(), 0, entry);
}

function pump() {
  while (active < MAX_WARMING && waiting.length > 0) {
    const entry = waiting.shift()!;
    active++;
    entry.start();
  }
}

async function load(src: string, priority: WarmPriority): Promise<void> {
  try {
    // the board may have shown it while it waited
    if (decoded.has(src)) return;
    const image = new Image();
    image.decoding = 'async';
    if (priority === 'low') (image as HTMLImageElement & { fetchPriority?: string }).fetchPriority = 'low';
    image.src = src;
    await image.decode();
    decoded.add(src);
    hold(image);
  } catch {
    // a picture that fails here is tried again by the card face that shows it
  } finally {
    active--;
    pump();
  }
}

/** Every size the app shows a card at: the device's picture store keeps them all (board, zoom, deck boxes). */
const STORED_SIZES: ImageSize[] = ['small', 'normal', 'large', 'art_crop'];

/**
 * The sizes a match shows a card at, in the order they are warmed: `small` for narrow permanents, piles and reveals,
 * `normal` for the hand, wide permanents and the stack, `large` for zoom and card detail.
 */
export const MATCH_SIZES: ImageSize[] = ['small', 'normal', 'large'];

/**
 * Warm every picture of these cards at the sizes the match shows (board and zoom). Links that are still being looked
 * up are warmed as soon as the resolver learns them. In the Android app every size is also stored on the device.
 * Each size is warmed for every card before the next size, so the board's pictures come before the zoom's.
 */
export function warmCards(cards: Iterable<CardImageRef>, sizes: ImageSize[] = MATCH_SIZES, priority: WarmPriority = 'high') {
  watchLinks(cards, sizes, (src) => void warmImage(src, priority));
}

/** A deck list entry, as stored (DeckCardInfo) or as sent to the server (WireDeck). */
interface DeckEntry {
  cardName?: string;
  setCode?: string | null;
  cardNumber?: string | null;
}

/** The cards of a deck (main deck and sideboard), as picture lookups. */
export function deckPictures(deck: { cards?: DeckEntry[]; sideboard?: DeckEntry[] } | null | undefined): CardImageRef[] {
  if (!deck) return [];
  return [...(deck.cards ?? []), ...(deck.sideboard ?? [])]
    .filter((card) => !!card.cardName)
    .map((card) => ({ name: card.cardName, setCode: card.setCode ?? undefined, cardNumber: card.cardNumber ?? undefined }));
}

/**
 * Warm a deck that is about to be played (yours or a known opponent's) at the match's sizes, at low priority: it
 * downloads while the table is set up, and gives way to whatever the screen is showing.
 */
export function warmDeck(deck: Parameters<typeof deckPictures>[0]) {
  const cards = deckPictures(deck);
  if (cards.length > 0) warmCards(cards, MATCH_SIZES, 'low');
}

/**
 * A deck the player is looking at, likely to be played next: its picture links are looked up now, so the match only
 * has to download them. The Android app also keeps the pictures on the device, without decoding them now.
 */
export function storeCards(cards: Iterable<CardImageRef>) {
  if (DEVICE_PICTURES) watchLinks(cards, [], () => undefined);
  else imageResolver().prefetch(cards);
}

/** Calls `onLink` with each card's link at each size as the resolver learns them; device storage rides along. */
function watchLinks(cards: Iterable<CardImageRef>, sizes: ImageSize[], onLink: (src: string) => void) {
  const resolver = imageResolver();
  const list = [...cards];
  resolver.prefetch(list);
  const remaining = new Set(list);
  const wanted = DEVICE_PICTURES ? [...new Set([...sizes, ...STORED_SIZES])] : sizes;
  const tryWarm = () => {
    const store: string[] = [];
    const found: string[][] = sizes.map(() => []);
    for (const card of remaining) {
      let resolved = true;
      for (const size of wanted) {
        const src = resolver.resolve(card, 'front', size);
        if (typeof src === 'string') {
          const index = sizes.indexOf(size);
          if (index >= 0) found[index].push(src);
          if (DEVICE_PICTURES) store.push(src);
        } else if (src === null) resolved = false;
      }
      if (resolved) remaining.delete(card);
    }
    for (const links of found) links.forEach(onLink);
    storePictures(store);
    return remaining.size === 0;
  };
  if (tryWarm()) return;
  const unsubscribe = resolver.subscribe(() => {
    if (tryWarm()) unsubscribe();
  });
  // links that never resolve stop being watched after a while
  setTimeout(unsubscribe, 60_000);
}

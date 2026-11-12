import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CardImageRef, ImageSize } from '../../core/images/imageLinks';

const resolver = {
  prefetch: vi.fn(),
  subscribe: vi.fn(() => () => undefined),
  resolve: vi.fn((card: CardImageRef, _face: string, size: ImageSize) => `/img/${card.name}-${size}.jpg`),
};
vi.mock('../../core/images/useCardImage', () => ({ imageResolver: () => resolver }));
vi.mock('../nativePictures', () => ({ DEVICE_PICTURES: false, storePictures: vi.fn() }));

/** Pictures the fake browser is downloading; each finishes when the test says so. */
let loads: { src: string; fetchPriority?: string; finish: () => void }[] = [];

class FakeImage {
  decoding = '';
  fetchPriority?: string;
  src = '';
  decode() {
    return new Promise<void>((resolve) => {
      loads.push({ src: this.src, fetchPriority: this.fetchPriority, finish: resolve });
    });
  }
}

async function freshCache() {
  vi.resetModules();
  return import('./imageCache');
}

/** lets queued promise callbacks run */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
  loads = [];
  vi.stubGlobal('Image', FakeImage);
  resolver.prefetch.mockClear();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('warmCards', () => {
  it('warms every size the match shows, smallest first for every card before the next size', async () => {
    const cache = await freshCache();
    expect(cache.MATCH_SIZES).toEqual(['small', 'normal', 'large']);
    cache.warmCards([{ name: 'A' }, { name: 'B' }]);
    await settle();
    // at most MAX_WARMING at once; finish them as they come
    const seen: string[] = [];
    while (loads.length > 0) {
      const batch = loads;
      loads = [];
      seen.push(...batch.map((load) => load.src));
      batch.forEach((load) => load.finish());
      await settle();
    }
    expect(seen).toEqual([
      '/img/A-small.jpg', '/img/B-small.jpg',
      '/img/A-normal.jpg', '/img/B-normal.jpg',
      '/img/A-large.jpg', '/img/B-large.jpg',
    ]);
    expect(cache.isDecoded('/img/B-small.jpg')).toBe(true);
  });
});

describe('warmImage', () => {
  it('downloads a few at a time and puts high priority ahead of low', async () => {
    const cache = await freshCache();
    for (let index = 0; index < 10; index++) void cache.warmImage(`/low/${index}`, 'low');
    void cache.warmImage('/high/0', 'high');
    await settle();
    expect(loads).toHaveLength(cache.MAX_WARMING);
    expect(loads.every((load) => load.fetchPriority === 'low')).toBe(true);
    loads.shift()!.finish();
    await settle();
    // the freed slot goes to the high-priority picture, at normal fetch priority
    const next = loads[loads.length - 1];
    expect(next.src).toBe('/high/0');
    expect(next.fetchPriority).toBeUndefined();
    expect(loads).toHaveLength(cache.MAX_WARMING);
  });

  it('moves a waiting low-priority picture up when the board asks for it', async () => {
    const cache = await freshCache();
    for (let index = 0; index < 8; index++) void cache.warmImage(`/low/${index}`, 'low');
    void cache.warmImage('/low/7', 'high');
    await settle();
    loads.shift()!.finish();
    await settle();
    expect(loads[loads.length - 1].src).toBe('/low/7');
  });

  it('skips pictures the board already showed while they waited', async () => {
    const cache = await freshCache();
    for (let index = 0; index < cache.MAX_WARMING; index++) void cache.warmImage(`/busy/${index}`, 'low');
    const later = cache.warmImage('/shown', 'low');
    cache.markDecoded('/shown');
    await settle();
    loads.shift()!.finish();
    await later;
    expect(loads.map((load) => load.src)).not.toContain('/shown');
  });

  it('does not download the same picture twice', async () => {
    const cache = await freshCache();
    void cache.warmImage('/same');
    void cache.warmImage('/same', 'low');
    await settle();
    expect(loads).toHaveLength(1);
  });
});

describe('deckPictures', () => {
  it('lists the main deck and the sideboard', async () => {
    const cache = await freshCache();
    expect(cache.deckPictures({
      cards: [{ cardName: 'Forest', setCode: 'M21', cardNumber: '274' }, { cardName: '' }],
      sideboard: [{ cardName: 'Duress', setCode: null, cardNumber: null }],
    })).toEqual([
      { name: 'Forest', setCode: 'M21', cardNumber: '274' },
      { name: 'Duress', setCode: undefined, cardNumber: undefined },
    ]);
    expect(cache.deckPictures(null)).toEqual([]);
  });

  it('warmDeck warms a deck at low priority', async () => {
    const cache = await freshCache();
    cache.warmDeck({ cards: [{ cardName: 'Forest' }], sideboard: [{ cardName: 'Duress' }] });
    await settle();
    expect(loads.map((load) => load.src)).toEqual(['/img/Forest-small.jpg', '/img/Duress-small.jpg', '/img/Forest-normal.jpg', '/img/Duress-normal.jpg']);
    expect(loads.every((load) => load.fetchPriority === 'low')).toBe(true);
  });
});

describe('storeCards', () => {
  it('looks the links up on the web without downloading the pictures', async () => {
    const cache = await freshCache();
    cache.storeCards([{ name: 'Forest' }]);
    await settle();
    expect(resolver.prefetch).toHaveBeenCalledWith([{ name: 'Forest' }]);
    expect(loads).toHaveLength(0);
  });
});

describe('isDecoded', () => {
  /** A browser image cache holding `cached`: those report complete as soon as their src is set. */
  function cacheHolding(cached: string[]) {
    const created: { src: string }[] = [];
    vi.stubGlobal('Image', class {
      private value = '';
      constructor() {
        created.push(this);
      }
      get src() {
        return this.value;
      }
      set src(value: string) {
        this.value = value;
      }
      get complete() {
        return cached.includes(this.value);
      }
      get naturalWidth() {
        return cached.includes(this.value) ? 488 : 0;
      }
    });
    return created;
  }

  it('shows a picture the browser already has on the first frame', async () => {
    cacheHolding(['/img/cached.jpg']);
    const { isDecoded } = await freshCache();
    expect(isDecoded('/img/cached.jpg')).toBe(true);
  });

  it('lets go of a picture the browser lacks at once, and asks only once', async () => {
    const created = cacheHolding([]);
    const { isDecoded } = await freshCache();
    expect(isDecoded('/img/new.jpg')).toBe(false);
    expect(created[0].src).toBe('');
    expect(isDecoded('/img/new.jpg')).toBe(false);
    expect(created).toHaveLength(1);
  });
});

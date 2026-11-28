import { describe, expect, test, vi } from 'vitest';
import { ImageResolver, type ImageLinkStore, type PrintingImages } from './ImageResolver';
import { exceptionLink, isTokenImage, normalizeCollectorNumber, printingKey, toImageLink, tokenImageName } from './imageLinks';

function memoryStore(initial: Record<string, PrintingImages> = {}): ImageLinkStore & { data: Map<string, PrintingImages> } {
  const data = new Map(Object.entries(initial));
  return {
    data,
    get: async (key) => data.get(key),
    setMany: async (entries) => {
      entries.forEach(([key, value]) => data.set(key, value));
    },
  };
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
}

const LINKS = {
  cards: {
    'SOI/Tamiyo\'s Journal/265+a': 'https://api.scryfall.com/cards/soi/265†a/',
    'ECL/Blood Crypt/349b': 'https://api.scryfall.com/cards/ecl/349/en?format=image&face=back',
  },
  tokens: {
    'RIX/Golem': 'https://api.scryfall.com/cards/trix/4/en?format=image',
    '10E/Soldier': 'https://api.scryfall.com/cards/t10e/1/en?format=image',
    'M19/Goblin/1': 'https://api.scryfall.com/cards/tm19/9/en?format=image',
    'M19/Goblin/2': 'https://api.scryfall.com/cards/tm19/10/en?format=image',
    'WAR/Emblem Nissa': 'https://api.scryfall.com/cards/twar/19/en?format=image',
    'PCA/Plane - Akoum': 'https://api.scryfall.com/cards/pca/57/en?format=image',
    'XMAGE/Copy/2': 'https://api.scryfall.com/cards/tsnc/1/en?format=image',
  },
};

function createResolver(fetchImpl: typeof fetch, store = memoryStore(), imageProxy?: string) {
  return new ImageResolver({ fetch: fetchImpl, store, batchDelayMs: 1, minRequestIntervalMs: 0, retryDelayMs: 1, imageProxy });
}

describe('ImageResolver', () => {
  test('goes through the image proxy when one is set, and keeps the stored links raw', async () => {
    const store = memoryStore({
      'm10/146': { front: { normal: 'https://cards.scryfall.io/normal/front/a/b/bolt.jpg?1' }, fetchedAt: Date.now() },
    });
    const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      expect(String(url)).toBe('/img/api/cards/collection');
      return jsonResponse({
        data: [{ set: 'isd', collector_number: '51', image_uris: { normal: 'https://cards.scryfall.io/normal/front/c/d/delver.jpg?2' } }],
      });
    }) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl, store, '/img');

    const bolt = { name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' };
    const delver = { name: 'Delver of Secrets', expansionSetCode: 'ISD', cardNumber: '51' };
    expect(resolver.resolve(bolt)).toBeNull();
    expect(resolver.resolve(delver)).toBeNull();
    await vi.waitFor(() => expect(resolver.resolve(delver)).not.toBeNull());

    // a link cached before (IndexedDB) and a freshly fetched one both come back proxied
    expect(resolver.resolve(bolt)).toBe('/img/cards/normal/front/a/b/bolt.jpg?1');
    expect(resolver.resolve(delver)).toBe('/img/cards/normal/front/c/d/delver.jpg?2');
    expect(store.data.get('isd/51')?.front?.normal).toBe('https://cards.scryfall.io/normal/front/c/d/delver.jpg?2');
    // API image links (exported lists, name lookups) too
    expect(resolver.resolve({ name: 'Golem', expansionSetCode: 'RIX', isToken: true }, 'front', 'large'))
      .toBe('/img/api/cards/trix/4/en?format=image&version=large');
  });

  test('looks printings up in one batch and serves faces and sizes', async () => {
    const fetchImpl = vi.fn(async (url: RequestInfo | URL, init?: RequestInit) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      const body = JSON.parse(String(init?.body));
      expect(body.identifiers).toEqual([
        { set: 'm10', collector_number: '146' },
        { set: 'isd', collector_number: '51' },
      ]);
      return jsonResponse({
        data: [
          { set: 'm10', collector_number: '146', image_uris: { normal: 'bolt-normal', art_crop: 'bolt-art' } },
          { set: 'isd', collector_number: '51', card_faces: [{ image_uris: { normal: 'front' } }, { image_uris: { normal: 'back' } }] },
        ],
      });
    }) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl);

    expect(resolver.resolve({ name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' })).toBeNull();
    expect(resolver.resolve({ name: 'Delver of Secrets', expansionSetCode: 'ISD', cardNumber: '51' })).toBeNull();
    await vi.waitFor(() => expect(resolver.getVersion()).toBeGreaterThan(1));

    expect(resolver.resolve({ name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' }, 'front', 'art_crop')).toBe('bolt-art');
    expect(resolver.resolve({ name: 'Delver of Secrets', expansionSetCode: 'ISD', cardNumber: '51' }, 'back')).toBe('back');
    const collectionCalls = (fetchImpl as unknown as { mock: { calls: unknown[][] } }).mock.calls
      .filter(([url]) => String(url).includes('/cards/collection'));
    expect(collectionCalls).toHaveLength(1);
  });

  test('uses exported links for tokens and special printings', async () => {
    const fetchImpl = vi.fn(async () => jsonResponse(LINKS)) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl);
    resolver.resolve({ name: 'x' });
    await vi.waitFor(() => expect(resolver.getVersion()).toBeGreaterThan(0));

    expect(resolver.resolve({ name: 'Golem', expansionSetCode: 'RIX', isToken: true }, 'front', 'large'))
      .toBe('https://api.scryfall.com/cards/trix/4/en?format=image&version=large');
    expect(resolver.resolve({ name: 'Unknown Token', expansionSetCode: 'RIX', isToken: true })).toBeUndefined();
  });

  test('falls back to a name lookup when Scryfall does not know a printing', async () => {
    const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      return jsonResponse({ data: [], not_found: [{}] });
    }) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl);
    resolver.resolve({ name: 'Custom Card', expansionSetCode: 'CUS', cardNumber: '1' });
    await vi.waitFor(() => expect(resolver.getVersion()).toBeGreaterThan(1));
    expect(resolver.resolve({ name: 'Custom Card', expansionSetCode: 'CUS', cardNumber: '1' }))
      .toContain('https://api.scryfall.com/cards/named?exact=Custom+Card');
  });

  test('retries a lookup the proxy or the network failed, instead of sending every card to Scryfall alone', async () => {
    let lookups = 0;
    const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      lookups++;
      if (lookups === 1) return jsonResponse({}, 502);
      if (lookups === 2) throw new TypeError('Failed to fetch');
      return jsonResponse({ data: [{ set: 'm10', collector_number: '146', image_uris: { normal: 'bolt' } }] });
    }) as unknown as typeof fetch;
    const resolver = new ImageResolver({ fetch: fetchImpl, store: memoryStore(), batchDelayMs: 1, minRequestIntervalMs: 0, retryDelayMs: 1 });
    const bolt = { name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' };
    resolver.resolve(bolt);
    await vi.waitFor(() => expect(resolver.resolve(bolt)).toBe('bolt'));
    expect(lookups).toBe(3);
  });

  test('gives up and clears inFlight on a non-JSON 200 response from the image proxy so the batch can retry later', async () => {
    let clock = 1_000_000;
    let lookups = 0;
    const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      lookups++;
      if (lookups === 1) return new Response('<html>proxy error</html>', { status: 200, headers: { 'Content-Type': 'text/html' } });
      return jsonResponse({ data: [{ set: 'm10', collector_number: '146', image_uris: { normal: 'bolt-recovered' } }] });
    }) as unknown as typeof fetch;
    const resolver = new ImageResolver({
      fetch: fetchImpl,
      store: memoryStore(),
      batchDelayMs: 1,
      minRequestIntervalMs: 0,
      retryDelayMs: 1,
      now: () => clock,
    });
    const bolt = { name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' };
    const listener = vi.fn();
    resolver.subscribeCard(bolt, listener);

    expect(resolver.resolve(bolt)).toBeNull();
    await vi.waitFor(() => expect(resolver.resolve(bolt)).toContain('cards/named?exact=Lightning+Bolt'));
    expect(listener).toHaveBeenCalled();

    // After the 1-minute backoff expires, the printing is no longer stuck in inFlight and retries cleanly
    clock += 61_000;
    expect(resolver.resolve(bolt)).toBeNull();
    await vi.waitFor(() => expect(resolver.resolve(bolt)).toBe('bolt-recovered'));
    expect(lookups).toBe(2);
  });

  test('reads the persistent cache before the network', async () => {
    const store = memoryStore({ 'm10/146': { front: { normal: 'cached' }, fetchedAt: Date.now() } });
    const fetchImpl = vi.fn(async () => jsonResponse(LINKS)) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl, store);
    resolver.resolve({ name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' });
    await vi.waitFor(() => expect(resolver.resolve({ name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' })).toBe('cached'));
    const collectionCalls = (fetchImpl as unknown as { mock: { calls: unknown[][] } }).mock.calls
      .filter(([url]) => String(url).includes('/cards/collection'));
    expect(collectionCalls).toHaveLength(0);
  });

  test('reads every stored link at start, so a known printing resolves without waiting', async () => {
    const store = memoryStore({
      'm10/146': { front: { normal: 'cached-bolt' }, fetchedAt: Date.now() },
      'm10/1': { front: { normal: 'old' }, fetchedAt: 0 },
    });
    const get = vi.spyOn(store, 'get');
    const withAll = { ...store, get, getAll: async () => [...store.data.entries()] };
    const fetchImpl = vi.fn(async () => jsonResponse(LINKS)) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl, withAll);
    await vi.waitFor(() => expect(resolver.getVersion()).toBeGreaterThan(0));
    await vi.waitFor(() => expect(resolver.resolve({ name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' })).toBe('cached-bolt'));
    expect(get).not.toHaveBeenCalledWith('m10/146');
    // a stale link is not taken: the card is looked up again
    expect(resolver.resolve({ name: 'Old Card', expansionSetCode: 'M10', cardNumber: '1' })).toBeNull();
  });

  test('normalizes XMage collector numbers like the desktop client', () => {
    expect(normalizeCollectorNumber('15*')).toBe('15★');
    expect(normalizeCollectorNumber('23+')).toBe('23†');
    expect(normalizeCollectorNumber('1Ph')).toBe('1Φ');
    expect(normalizeCollectorNumber(' 146 ')).toBe('146');
    expect(printingKey({ expansionSetCode: 'PBNG', cardNumber: '15*' })).toBe('pbng/15★');
    expect(printingKey({ expansionSetCode: 'M10', cardNumber: '146' })).toBe('m10/146');
    expect(printingKey({ expansionSetCode: '', cardNumber: '1' })).toBeNull();
  });

  test('looks promo printings up with Scryfall collector numbers', async () => {
    const fetchImpl = vi.fn(async (url: RequestInfo | URL, init?: RequestInit) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      const body = JSON.parse(String(init?.body));
      expect(body.identifiers).toEqual([{ set: 'pbng', collector_number: '15★' }]);
      return jsonResponse({ data: [{ set: 'pbng', collector_number: '15★', image_uris: { normal: 'promo' } }] });
    }) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl);
    const promo = { name: 'Arbiter of the Ideal', expansionSetCode: 'PBNG', cardNumber: '15*' };
    expect(resolver.resolve(promo)).toBeNull();
    await vi.waitFor(() => expect(resolver.resolve(promo)).toBe('promo'));
  });

  test('serves tokens, emblems, planes and XMage pictures from the token lists', async () => {
    const fetchImpl = vi.fn(async () => jsonResponse(LINKS)) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl);
    // a plane before the lists load waits instead of guessing a name lookup
    expect(resolver.resolve({ name: 'Plane - Akoum', expansionSetCode: 'PCA' })).toBeNull();
    await vi.waitFor(() => expect(resolver.getVersion()).toBeGreaterThan(0));

    // the server names tokens "Soldier Token", the lists "Soldier"
    expect(resolver.resolve({ name: 'Soldier Token', expansionSetCode: '10E', cardNumber: '', isToken: true, mageObjectType: 'TOKEN' }))
      .toBe('https://api.scryfall.com/cards/t10e/1/en?format=image&version=normal');
    expect(resolver.resolve({ name: 'Goblin Token', expansionSetCode: 'M19', isToken: true, imageNumber: 2 }))
      .toBe('https://api.scryfall.com/cards/tm19/10/en?format=image&version=normal');
    // emblem and plane views carry no object type or collector number
    expect(resolver.resolve({ name: 'Emblem Nissa', expansionSetCode: 'WAR', imageNumber: 0 }))
      .toBe('https://api.scryfall.com/cards/twar/19/en?format=image&version=normal');
    expect(resolver.resolve({ name: 'Plane - Akoum', expansionSetCode: 'PCA' }))
      .toBe('https://api.scryfall.com/cards/pca/57/en?format=image&version=normal');
    // XMage's own pictures (copies, face down...) are named by the image file name
    expect(resolver.resolve({ name: 'Copy', imageFileName: 'Copy', expansionSetCode: 'XMAGE', cardNumber: '0', imageNumber: 2 }))
      .toBe('https://api.scryfall.com/cards/tsnc/1/en?format=image&version=normal');
    // unknown command objects fall back to a name lookup
    expect(resolver.resolve({ name: 'Emblem Unknown', expansionSetCode: 'WAR' })).toContain('cards/named');
    const collectionCalls = (fetchImpl as unknown as { mock: { calls: unknown[][] } }).mock.calls
      .filter(([url]) => String(url).includes('/cards/collection'));
    expect(collectionCalls).toHaveLength(0);
  });

  test('a token copy of a card shows the card printing', async () => {
    const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
      if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
      return jsonResponse({ data: [{ set: 'm10', collector_number: '146', image_uris: { normal: 'bolt' } }] });
    }) as unknown as typeof fetch;
    const resolver = createResolver(fetchImpl);
    const copy = { name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146', isToken: true, mageObjectType: 'TOKEN' };
    expect(isTokenImage(copy)).toBe(false);
    resolver.resolve(copy);
    await vi.waitFor(() => expect(resolver.resolve(copy)).toBe('bolt'));
  });

  test('builds image links from the exported lists', () => {
    expect(toImageLink('https://api.scryfall.com/cards/dis/176/', 'large'))
      .toBe('https://api.scryfall.com/cards/dis/176?format=image&version=large');
    expect(exceptionLink({ name: 'Tamiyo\'s Journal', expansionSetCode: 'SOI', cardNumber: '265+a' }, 'front', LINKS))
      .toBe('https://api.scryfall.com/cards/soi/265†a/');
    expect(toImageLink(LINKS.cards['ECL/Blood Crypt/349b'], 'normal'))
      .toBe('https://api.scryfall.com/cards/ecl/349/en?format=image&face=back&version=normal');
    expect(exceptionLink({ name: 'Blood Crypt', expansionSetCode: 'ECL', cardNumber: '349' }, 'back', LINKS))
      .toBe(LINKS.cards['ECL/Blood Crypt/349b']);
    expect(tokenImageName({ name: 'Goblin Token' })).toBe('Goblin');
    expect(tokenImageName({ name: '', imageFileName: 'Face Down' })).toBe('Face Down');
    expect(isTokenImage({ name: 'Face Down', expansionSetCode: 'XMAGE', cardNumber: '0' })).toBe(true);
    expect(isTokenImage({ name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' })).toBe(false);
  });

  describe('notifications', () => {
    const BOLT = { name: 'Lightning Bolt', expansionSetCode: 'M10', cardNumber: '146' };
    const DELVER = { name: 'Delver of Secrets', expansionSetCode: 'ISD', cardNumber: '51' };
    const GOLEM = { name: 'Golem', expansionSetCode: 'RIX', isToken: true };

    function deferredLinks() {
      let release: () => void = () => undefined;
      const linksReady = new Promise<void>((resolve) => { release = resolve; });
      return { linksReady, release: () => release() };
    }

    test('only the subscribers of a printing hear about its links', async () => {
      const { linksReady, release } = deferredLinks();
      // Bolt comes from the persistent cache, Delver from the network
      const store = memoryStore({ 'm10/146': { front: { normal: 'cached-bolt' }, fetchedAt: Date.now() } });
      const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
        if (String(url).endsWith('scryfall-links.json')) {
          await linksReady;
          return jsonResponse(LINKS);
        }
        return jsonResponse({ data: [{ set: 'isd', collector_number: '51', image_uris: { normal: 'delver' } }] });
      }) as unknown as typeof fetch;
      const resolver = createResolver(fetchImpl, store);
      const bolt = vi.fn();
      const delver = vi.fn();
      const token = vi.fn();
      const any = vi.fn();
      resolver.subscribeCard(BOLT, bolt);
      resolver.subscribeCard(DELVER, delver);
      resolver.subscribeCard(GOLEM, token);
      resolver.subscribe(any);

      resolver.resolve(BOLT);
      resolver.resolve(DELVER);
      await vi.waitFor(() => expect(resolver.resolve(BOLT)).toBe('cached-bolt'));
      await vi.waitFor(() => expect(resolver.resolve(DELVER)).toBe('delver'));
      expect(bolt).toHaveBeenCalledTimes(1);
      expect(delver).toHaveBeenCalledTimes(1);
      expect(token).not.toHaveBeenCalled();
      expect(any).toHaveBeenCalledTimes(2);

      // the exported link lists can change any card (exceptions, tokens): everyone hears about them once
      release();
      await vi.waitFor(() => expect(token).toHaveBeenCalledTimes(1));
      expect(bolt).toHaveBeenCalledTimes(2);
      expect(delver).toHaveBeenCalledTimes(2);
      expect(any).toHaveBeenCalledTimes(3);
    });

    test('a listener subscribed to several channels is called once per change', async () => {
      const fetchImpl = vi.fn(async () => jsonResponse(LINKS)) as unknown as typeof fetch;
      const resolver = createResolver(fetchImpl);
      const listener = vi.fn();
      resolver.subscribeKey('m10/146', listener);
      resolver.subscribeKey(null, listener);
      resolver.subscribe(listener);
      resolver.resolve(GOLEM);
      await vi.waitFor(() => expect(listener).toHaveBeenCalled());
      await Promise.resolve();
      expect(listener).toHaveBeenCalledTimes(1);
    });

    test('unsubscribing stops notifications and failed lookups still notify their printing', async () => {
      const fetchImpl = vi.fn(async (url: RequestInfo | URL) => {
        if (String(url).endsWith('scryfall-links.json')) return jsonResponse(LINKS);
        throw new Error('offline');
      }) as unknown as typeof fetch;
      const resolver = createResolver(fetchImpl);
      const gone = vi.fn();
      const delver = vi.fn();
      const unsubscribe = resolver.subscribeCard(BOLT, gone);
      unsubscribe();
      resolver.subscribeCard(DELVER, delver);
      resolver.resolve(BOLT);
      resolver.resolve(DELVER);
      await vi.waitFor(() => expect(resolver.resolve(DELVER)).toContain('cards/named'));
      expect(gone).not.toHaveBeenCalled();
      expect(delver).toHaveBeenCalled();
    });
  });
});

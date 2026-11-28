import type { ImageLinkStore, PrintingImages } from './ImageResolver';

const DB_NAME = 'xmage-images';
const STORE = 'printings';

/** Image link cache in IndexedDB; does nothing (and never throws) when IndexedDB is unavailable. */
export function createIndexedDbImageStore(): ImageLinkStore {
  let dbPromise: Promise<IDBDatabase | null> | null = null;

  const open = (): Promise<IDBDatabase | null> => {
    if (dbPromise) return dbPromise;
    dbPromise = new Promise((resolve) => {
      try {
        if (typeof indexedDB === 'undefined') {
          resolve(null);
          return;
        }
        const request = indexedDB.open(DB_NAME, 1);
        request.onupgradeneeded = () => request.result.createObjectStore(STORE);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => resolve(null);
        request.onblocked = () => resolve(null);
      } catch {
        resolve(null);
      }
    });
    return dbPromise;
  };

  return {
    async get(key: string): Promise<PrintingImages | undefined> {
      const db = await open();
      if (!db) return undefined;
      return new Promise((resolve) => {
        try {
          const request = db.transaction(STORE, 'readonly').objectStore(STORE).get(key);
          request.onsuccess = () => resolve(request.result as PrintingImages | undefined);
          request.onerror = () => resolve(undefined);
        } catch {
          resolve(undefined);
        }
      });
    },

    async getAll(): Promise<[string, PrintingImages][]> {
      const db = await open();
      if (!db) return [];
      return new Promise((resolve) => {
        try {
          // two bulk reads instead of a cursor: one event per store, not one per printing
          const store = db.transaction(STORE, 'readonly').objectStore(STORE);
          const keys = store.getAllKeys();
          const values = store.getAll();
          values.onsuccess = () => {
            const found = values.result as PrintingImages[];
            resolve((keys.result ?? []).map((key, index): [string, PrintingImages] => [String(key), found[index]]));
          };
          values.onerror = () => resolve([]);
        } catch {
          resolve([]);
        }
      });
    },

    async setMany(entries: [string, PrintingImages][]): Promise<void> {
      const db = await open();
      if (!db || entries.length === 0) return;
      await new Promise<void>((resolve) => {
        try {
          const transaction = db.transaction(STORE, 'readwrite');
          const store = transaction.objectStore(STORE);
          for (const [key, value] of entries) store.put(value, key);
          transaction.oncomplete = () => resolve();
          transaction.onerror = () => resolve();
          transaction.onabort = () => resolve();
        } catch {
          resolve();
        }
      });
    },
  };
}

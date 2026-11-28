import { describe, expect, it } from 'vitest';
import { shuffledLibrary } from './libraryList';

describe('shuffledLibrary', () => {
  it('keeps every name and count, drops empty ones', () => {
    const list = shuffledLibrary({ Forest: 8, 'Grizzly Bears': 2, Gone: 0 });
    expect(list).toHaveLength(2);
    expect(Object.fromEntries(list.map((entry) => [entry.name, entry.count]))).toEqual({ Forest: 8, 'Grizzly Bears': 2 });
  });

  it('does not keep the order it was sent in', () => {
    const cards = Object.fromEntries(Array.from({ length: 12 }, (_, i) => [`Card ${String(i).padStart(2, '0')}`, 1]));
    const orders = new Set(Array.from({ length: 20 }, () => shuffledLibrary(cards).map((entry) => entry.name).join('|')));
    expect(orders.size).toBeGreaterThan(1);
  });

  it('is empty without a list (an opponent, a watcher)', () => {
    expect(shuffledLibrary(undefined)).toEqual([]);
  });
});

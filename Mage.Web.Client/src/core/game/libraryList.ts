/**
 * The player's own library for the library viewer. The server sends only names and counts
 * (PlayerView.libraryCards, in the player's own view, sorted by name), never the order; the list is shuffled here
 * each time it is opened so the screen does not suggest an order either.
 */
export interface LibraryEntry {
  name: string;
  count: number;
}

export function shuffledLibrary(cards: Readonly<Record<string, number>> | null | undefined, random: () => number = Math.random): LibraryEntry[] {
  const entries = Object.entries(cards ?? {}).filter(([, count]) => count > 0).map(([name, count]) => ({ name, count }));
  for (let i = entries.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [entries[i], entries[j]] = [entries[j], entries[i]];
  }
  return entries;
}

import type { CardCriteria, CardView } from '../../protocol/generated/views';

/** The server's largest page (CardSearchService.MAX_PAGE_SIZE, playmat-platform search MAX_PAGE). */
export const PRINTINGS_PAGE = 1000;
/** A guard against a server that keeps answering full pages: no card has this many printings. */
const MAX_PAGES = 10;

/**
 * Every printing of a card, page by page. A basic land has over 800 printings, more than one page
 * would hold, and the rows come in database order, so a single page left out some newer sets' basics.
 */
export async function allPrintings(name: string, search: (criteria: CardCriteria) => Promise<CardView[]>): Promise<CardView[]> {
  const found: CardView[] = [];
  for (let page = 0; page < MAX_PAGES; page++) {
    const rows = await search({ name, uniqueNames: false, start: page * PRINTINGS_PAGE, count: PRINTINGS_PAGE });
    found.push(...rows);
    if (rows.length < PRINTINGS_PAGE) break;
  }
  return found;
}

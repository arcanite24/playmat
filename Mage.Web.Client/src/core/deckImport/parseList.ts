import { DeckSerializer } from '../decks/DeckSerializer';
import type { DeckCardInfo } from '../decks/types';

export type ListSection = 'main' | 'side' | 'commander' | 'companion' | 'maybe';

/** A deck list read from text, before any card is matched against the card database. */
export interface ParsedList {
  name?: string;
  main: DeckCardInfo[];
  side: DeckCardInfo[];
  commanders: DeckCardInfo[];
  companion: DeckCardInfo[];
  /** cards the player keeps "maybe" lists of; never imported */
  maybe: DeckCardInfo[];
  /** lines that began with a count but weren't a card we could read; shown to the player, never imported */
  skipped: string[];
}

const SECTION_NAMES: Record<string, ListSection> = {
  deck: 'main', main: 'main', maindeck: 'main', 'main deck': 'main', mainboard: 'main',
  sideboard: 'side', side: 'side', sb: 'side',
  commander: 'commander', commanders: 'commander', 'command zone': 'commander', commandzone: 'commander',
  companion: 'companion', companions: 'companion',
  maybeboard: 'maybe', maybe: 'maybe', considering: 'maybe', acquireboard: 'maybe', 'tokens & extras': 'maybe', tokens: 'maybe',
};

// headers that group cards by type; they never change the zone
const TYPE_HEADERS = /^(creatures?|lands?|instants?|sorcery|sorceries|artifacts?|enchantments?|planeswalkers?|battles?|spells?|other spells|other|non-?creature spells?|nonlands|ramp|removal|draw)\s*(\(\d+\))?$/i;

/** "Sideboard", "// Sideboard", "SIDEBOARD:", "Sideboard (15)" → the section it opens. */
function sectionOf(line: string): ListSection | 'about' | null {
  const header = line.replace(/^\/\/\s*/, '').replace(/\s*\(\d+\)\s*$/, '').replace(/:\s*$/, '').trim().toLowerCase();
  if (header === 'about') return 'about';
  return SECTION_NAMES[header] ?? null;
}

/** Split cards and flip cards are "A // B"; exports write them a few ways. */
export function normalizeCardName(name: string): string {
  return name
    .replace(/&amp;/g, '&')
    .replace(/\s*\/{1,3}\s*/g, ' // ')
    .replace(/\s+/g, ' ')
    .trim();
}

interface CardLine {
  card: DeckCardInfo;
  sideboard: boolean;
  /** a marker on the line itself put it in this section (Moxfield's *CMDR*, Archidekt's [Commander]) */
  section?: ListSection;
}

/** One card line in any of the common export formats; null when the line isn't a card. */
export function parseCardLine(raw: string): CardLine | null {
  let line = raw.trim();
  let section: ListSection | undefined;

  // Archidekt colors "^Have,#37d67a^" and finish markers "*F*" "*E*"; "*CMDR*" marks a commander
  line = line.replace(/\^[^^]*\^/g, ' ');
  line = line.replace(/\*([A-Za-z]+)\*/g, (_match, marker: string) => {
    if (/^cmdr$/i.test(marker)) section = 'commander';
    return ' ';
  });
  // Archidekt categories at the end: "[Commander{top}]", "[Maybeboard{noDeck}{noPrice},Ramp]"
  line = line.replace(/\s+\[([^\]:]*)\]\s*$/, (_match, tags: string) => {
    if (/(^|,)\s*commander\b/i.test(tags)) section = 'commander';
    else if (/(^|,)\s*maybeboard\b/i.test(tags) || /\{noDeck\}/i.test(tags)) section = 'maybe';
    else if (/(^|,)\s*sideboard\b/i.test(tags)) section = 'side';
    return '';
  });
  line = line.replace(/\s+/g, ' ').trim();

  let sideboard = false;
  if (/^SB:\s*/i.test(line)) {
    sideboard = true;
    line = line.replace(/^SB:\s*/i, '');
  }

  const counted = /^(\d+)\s*x?\s+(.+)$/i.exec(line);
  if (!counted) return null;
  const amount = Number.parseInt(counted[1], 10);
  if (!amount) return null;
  let rest = counted[2].trim();
  let setCode: string | null = null;
  let cardNumber: string | null = null;

  const xmage = /^\[([^:\]]+):([^\]]+)\]\s*(.+)$/.exec(rest);
  const setOnly = /^\[([A-Za-z0-9]*)\]\s*(.+)$/.exec(rest);
  if (xmage) {
    setCode = xmage[1];
    cardNumber = xmage[2];
    rest = xmage[3];
  } else if (setOnly) {
    // MTGTop8 and MWS: "4 [M11] Lightning Bolt"; a set without a number only helps the lookup pick a printing
    setCode = setOnly[1] || null;
    rest = setOnly[2];
  } else {
    // Arena and friends: "Name (SET) 123", "Name (SET)"; collector numbers can hold letters, ★ and dashes
    const arena = /^(.+?)\s+\(([A-Za-z0-9]{2,6})\)(?:\s+([A-Za-z0-9★†-]+))?$/.exec(rest);
    if (arena) {
      rest = arena[1];
      setCode = arena[2];
      cardNumber = arena[3] ?? null;
    }
  }
  const cardName = normalizeCardName(rest.replace(/\s*\((?:F|foil|etched)\)\s*$/i, ''));
  if (!cardName) return null;
  return {
    card: { amount, cardName, setCode: setCode ? setCode.toUpperCase() : null, cardNumber },
    sideboard,
    section,
  };
}

const empty = (): ParsedList => ({ main: [], side: [], commanders: [], companion: [], maybe: [], skipped: [] });

function push(list: ParsedList, section: ListSection, card: DeckCardInfo) {
  const target = { main: list.main, side: list.side, commander: list.commanders, companion: list.companion, maybe: list.maybe }[section];
  target.push(card);
}

/** Files the old serializer already reads well: XML (Cockatrice, OCTGN), JSON (MTGJSON) and XMage draft logs. */
function isDocument(text: string): boolean {
  return /^\s*(<|\{)/.test(text) || (/^\s*------ [A-Za-z0-9]+ ------\s*$/m.test(text) && /^\s*-->\s+.+$/m.test(text));
}

/**
 * Reads a pasted or exported deck list into its sections.
 * Understands Arena, MTGO, XMage .dck, MTGTop8 .dec, Moxfield, Archidekt, TappedOut and Deckstats text exports.
 * Without section headers, a blank line between two blocks marks the sideboard (MTGO style).
 */
export function parseList(text: string): ParsedList {
  if (isDocument(text)) {
    const deck = DeckSerializer.importDeck(text);
    const result = empty();
    result.main = deck.cards;
    result.side = deck.sideboard;
    if (deck.name && deck.name !== 'Imported Deck') result.name = deck.name;
    return result;
  }

  const lines = text.split(/\r?\n/);
  const hasHeaders = lines.some((line) => {
    const section = sectionOf(line.trim());
    return section !== null && section !== 'about';
  });

  const result = empty();
  let section: ListSection = 'main';
  let about = false;
  let blankAfterCards = false;
  let blocks = 0;

  for (const raw of lines) {
    const line = raw.trim();
    if (!line) {
      if (!hasHeaders && result.main.length > 0 && section === 'main') blankAfterCards = true;
      continue;
    }
    if (line.startsWith('#') || (line.startsWith('//') && !sectionOf(line))) continue;

    const header = sectionOf(line);
    if (header === 'about') {
      about = true;
      continue;
    }
    if (header) {
      section = header;
      about = false;
      continue;
    }
    // XMage .dck and Arena "About" blocks carry the deck name
    const named = /^name\s*[:\s]\s*(.+)$/i.exec(line);
    if (named && (about || /^name:/i.test(line))) {
      result.name = named[1].trim();
      continue;
    }
    if (/^LAYOUT\s/i.test(line) || TYPE_HEADERS.test(line)) continue;

    const parsed = parseCardLine(line) ?? (/^\d/.test(line) ? null : { card: { amount: 1, cardName: normalizeCardName(line), setCode: null, cardNumber: null }, sideboard: false });
    if (!parsed) {
      result.skipped.push(line);
      continue;
    }
    about = false;

    if (blankAfterCards) {
      section = 'side';
      blankAfterCards = false;
      blocks += 1;
    }
    push(result, parsed.section ?? (parsed.sideboard ? 'side' : section), parsed.card);
  }

  // MTGO-style commander export: 99 cards, a blank line, then the commander or partners
  if (!hasHeaders && blocks === 1 && result.commanders.length === 0 && result.side.length > 0) {
    const mainCount = result.main.reduce((sum, card) => sum + card.amount, 0);
    const sideCount = result.side.reduce((sum, card) => sum + card.amount, 0);
    if (sideCount <= 2 && mainCount + sideCount >= 99 && mainCount + sideCount <= 101) {
      result.commanders = result.side;
      result.side = [];
    }
  }
  return result;
}

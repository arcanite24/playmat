import { createContext } from 'react';
import type { CardImageRef } from '../../core/images/imageLinks';

/**
 * Basic land styles (B301): a player who earned a land style sees their basic lands as that set's full-art printing,
 * and so does everyone else at the table. The swap happens only where a card is drawn: the deck, the game and the
 * card's own set and number are untouched.
 *
 * How it reaches the picture: pictures are looked up by set code and collector number (imageLinks.printingKey), and
 * that key is what the image resolver caches (IndexedDB) and what the deployment's imgcache proxy stores, so a restyled
 * Forest is just another printing to them: ZEN 246 resolves, caches and proxies like any card, with no name lookups
 * and no extra Scryfall searches. The collector numbers below are the full-art basics of each set as XMage's own set
 * files list them (Mage.Sets, FULL_ART_* graphic info), so every one of them is a printing the server and Scryfall
 * both know. A set without a basic of some name (none of these print Wastes) leaves that land as it is.
 */

export type BasicName = 'Plains' | 'Island' | 'Swamp' | 'Mountain' | 'Forest';

const range = (from: number, count: number) => Array.from({ length: count }, (_, i) => String(from + i));

export const LAND_STYLE_PRINTINGS: Record<string, Partial<Record<BasicName, readonly string[]>>> = {
  ZEN: { Plains: range(230, 4), Island: range(234, 4), Swamp: range(238, 4), Mountain: range(242, 4), Forest: range(246, 4) },
  BFZ: { Plains: range(250, 5), Island: range(255, 5), Swamp: range(260, 5), Mountain: range(265, 5), Forest: range(270, 5) },
  AKH: { Plains: ['250'], Island: ['251'], Swamp: ['252'], Mountain: ['253'], Forest: ['254'] },
  HOU: { Plains: ['185'], Island: ['186'], Swamp: ['187'], Mountain: ['188'], Forest: ['189'] },
  UST: { Plains: ['212'], Island: ['213'], Swamp: ['214'], Mountain: ['215'], Forest: ['216'] },
  UNH: { Plains: ['136'], Island: ['137'], Swamp: ['138'], Mountain: ['139'], Forest: ['140'] },
  ZNR: { Plains: range(266, 3), Island: range(269, 3), Swamp: range(272, 3), Mountain: range(275, 3), Forest: range(278, 3) },
  NEO: { Plains: range(293, 2), Island: range(295, 2), Swamp: range(297, 2), Mountain: range(299, 2), Forest: range(301, 2) },
};

/** A card as the styler sees it: its picture reference plus who owns and controls it. */
export type StylableCard = CardImageRef & { id?: string; cardId?: string; ownerId?: string; controllerId?: string; faceDown?: boolean };

/** The land style (a set code) of a card's player, or undefined: given by the match, null everywhere else. */
export type LandStyleOf = (playerId: string) => string | undefined;

export const LandStyleContext = createContext<LandStyleOf | null>(null);

/** A small stable hash, so each land keeps one of its set's arts for the whole game. */
function pick(key: string, count: number): number {
  let hash = 0;
  for (let i = 0; i < key.length; i++) hash = (hash * 31 + key.charCodeAt(i)) | 0;
  return Math.abs(hash) % count;
}

/**
 * The card to draw for a basic land in the set `style`: the same card with that set's full-art printing, chosen by the
 * card's id among the set's arts for its name. Anything else (not a basic, a token, a face-down card, a set without
 * that basic) comes back as it is.
 */
export function styledLand<T extends StylableCard>(card: T, style: string | undefined): T {
  if (!style || card.isToken || card.faceDown) return card;
  const name = (card.name ?? '') as BasicName;
  const numbers = LAND_STYLE_PRINTINGS[style.toUpperCase()]?.[name];
  if (!numbers || numbers.length === 0) return card;
  const cardNumber = numbers[pick(card.cardId ?? card.id ?? name, numbers.length)];
  const setCode = style.toUpperCase();
  if ((card.expansionSetCode ?? card.setCode) === setCode && card.cardNumber === cardNumber) return card;
  return { ...card, expansionSetCode: setCode, setCode, cardNumber };
}

/** The player whose land style a card takes: its owner's (it came from their deck), else its controller's. */
export function landStylist(card: StylableCard): string | undefined {
  return card.ownerId ?? card.controllerId;
}

import { createContext } from 'react';
import { isTokenImage, tokenImageName, type CardImageRef } from '../../core/images/imageLinks';

/**
 * Token art (#226): a player picks which picture each token shows (a Goblin from Magic 2010 rather than Dominaria), for
 * their account and, over that, for one deck. The table sees each player's tokens the way that player chose, like land
 * styles (landStyles.ts): the swap happens only where a card is drawn.
 *
 * A choice is the token's name (as the token lists name it, without " Token") to its picture in the lists exported
 * from the desktop client (/data/scryfall-links.json): "SET/Name" or "SET/Name/number". Drawing a token with a choice
 * gives it that set code and image number, and the image resolver finds the picture as usual.
 */

/** Token name to its chosen picture. */
export type TokenChoices = Record<string, string>;

/** A player's token choices in a match (account's, with the deck's over them), or undefined: given by the match, null elsewhere. */
export type TokenArtOf = (playerId: string) => TokenChoices | undefined;

export const TokenArtContext = createContext<TokenArtOf | null>(null);

/** A picture key's parts: "M10/Goblin/2" is set M10, name Goblin, image 2 (0 when the set has one picture). */
export function parsePicture(picture: string): { set: string; name: string; imageNumber: number } | null {
  const parts = picture.split('/');
  if (parts.length < 2 || parts.length > 3 || !parts[0] || !parts[1]) return null;
  const imageNumber = parts.length === 3 ? Number(parts[2]) : 0;
  if (!Number.isInteger(imageNumber) || imageNumber < 0) return null;
  return { set: parts[0], name: parts[1], imageNumber };
}

/** The account's choices with the deck's over them; undefined when neither has any. */
export function mergeChoices(account: TokenChoices | null | undefined, deck: TokenChoices | null | undefined): TokenChoices | undefined {
  const merged = { ...(account ?? {}), ...(deck ?? {}) };
  return Object.keys(merged).length > 0 ? merged : undefined;
}

/**
 * The card to draw for a token with these choices: the same token with the chosen set and image number. Anything else
 * (not a token picture, face down, no choice for its name) comes back as it is.
 */
export function styledToken<T extends CardImageRef & { faceDown?: boolean }>(card: T, choices: TokenChoices | undefined): T {
  if (!choices || card.faceDown || !isTokenImage(card)) return card;
  const picture = choices[tokenImageName(card)];
  const parsed = picture ? parsePicture(picture) : null;
  if (!parsed) return card;
  if ((card.expansionSetCode ?? card.setCode) === parsed.set && (card.imageNumber ?? 0) === parsed.imageNumber) return card;
  return { ...card, expansionSetCode: parsed.set, setCode: parsed.set, imageNumber: parsed.imageNumber };
}

/** A token picture as a card to draw (the picker's thumbnails). */
export function pictureCard(picture: string): CardImageRef | null {
  const parsed = parsePicture(picture);
  if (!parsed) return null;
  return { name: parsed.name, expansionSetCode: parsed.set, setCode: parsed.set, imageNumber: parsed.imageNumber, isToken: true, mageObjectType: 'TOKEN', cardNumber: '' };
}

/** The tokens decks make most, offered first in the picker. */
export const COMMON_TOKENS = [
  'Treasure', 'Food', 'Clue', 'Blood', 'Map', 'Goblin', 'Zombie', 'Spirit', 'Soldier', 'Human Soldier', 'Saproling', 'Elf Warrior',
  'Angel', 'Beast', 'Insect', 'Bird', 'Cat', 'Dragon', 'Thopter', 'Servo', 'Construct', 'Elemental', 'Wolf', 'Squirrel',
  'Rat', 'Faerie', 'Knight', 'Vampire', 'Plant', 'Germ', 'Eldrazi Spawn', 'Eldrazi Scion', 'Human', 'Shapeshifter', 'Copy',
];

/**
 * Card image addressing for Scryfall.
 *
 * Printings are identified by XMage set code + collector number (Scryfall uses the same codes, lower case).
 * Exceptions (special sets, tokens) come from the desktop client's lists, exported to /data/scryfall-links.json
 * by Mage.Client's WebClientImageLinksExportTest. The rules below mirror the desktop client's ScryfallImageSource
 * and CardImageUtils.buildImagePathToCardView, so both clients pick the same picture.
 */

export type ImageSize = 'small' | 'normal' | 'large' | 'art_crop' | 'png';
export type CardFace = 'front' | 'back';

/** What the image service needs to know about a card (CardView, PermanentView and deck cards all fit). */
export interface CardImageRef {
  name?: string;
  displayName?: string;
  expansionSetCode?: string;
  setCode?: string;
  cardNumber?: string;
  isToken?: boolean;
  imageNumber?: number;
  /** emblems, planes and other command objects are served like tokens */
  imageFileName?: string;
  /** server object type (TOKEN, EMBLEM, PLANE...); command object views do not send it */
  mageObjectType?: string;
  /** the back face of a double-faced card is up (transformed, or an MDFC played as its back) */
  backFace?: boolean;
  transformed?: boolean;
}

/** Both faces of a double-faced card share one collector number: the view says which one is up. */
export function showsBackFace(card: CardImageRef): boolean {
  return !!(card.backFace || card.transformed);
}

/** Set code of XMage's own pictures (face down, copy, monarch, day/night...), always from the token lists. */
export const XMAGE_TOKENS_SET_CODE = 'XMAGE';

/** Object types whose pictures come from the token lists (MageObjectType.isUseTokensRepository on the server). */
const TOKEN_IMAGE_TYPES = new Set(['TOKEN', 'ABILITY_STACK_FROM_TOKEN', 'DUNGEON', 'EMBLEM', 'COMMANDER', 'DESIGNATION', 'PLANE']);

export interface ScryfallLinks {
  cards: Record<string, string>;
  tokens: Record<string, string>;
}

export function setCodeOf(card: CardImageRef): string {
  return (card.expansionSetCode || card.setCode || '').trim();
}

/**
 * XMage collector number to Scryfall's (ScryfallApiCard.transformCardNumberFromXmageToScryfall): XMage writes the
 * star of promo printings as '*', the dagger as '+' and phi as 'Ph', e.g. PBNG 15* is Scryfall's pbng/15★.
 */
export function normalizeCollectorNumber(cardNumber: string | undefined): string {
  const number = (cardNumber ?? '').trim();
  if (number.endsWith('*')) return `${number.slice(0, -1)}★`;
  if (number.endsWith('+')) return `${number.slice(0, -1)}†`;
  if (number.endsWith('Ph')) return `${number.slice(0, -2)}Φ`;
  return number;
}

/**
 * Whether the picture comes from the token lists rather than a card printing: tokens, emblems, planes, dungeons
 * and XMage's own pictures. A token copy of a card keeps the card's collector number and shows that printing.
 */
export function isTokenImage(card: CardImageRef): boolean {
  if (setCodeOf(card).toUpperCase() === XMAGE_TOKENS_SET_CODE) return true;
  if (!card.isToken && !TOKEN_IMAGE_TYPES.has(card.mageObjectType ?? '')) return false;
  const number = (card.cardNumber ?? '').trim();
  return number === '' || number === '0';
}

/** Name of the picture in the token lists: the image file name if any, without the " Token" suffix. */
export function tokenImageName(card: CardImageRef): string {
  return (card.imageFileName || card.name || card.displayName || '').split(' Token').join('').trim();
}

export function printingKey(card: CardImageRef): string | null {
  const set = setCodeOf(card).toLowerCase();
  const number = normalizeCollectorNumber(card.cardNumber).toLowerCase();
  if (!set || !number) return null;
  return `${set}/${number}`;
}

/** Turns a Scryfall API link from the exported lists into an image link of the wanted size. */
export function toImageLink(apiLink: string, size: ImageSize): string {
  if (/\.(jpg|jpeg|png)(\?|$)/i.test(apiLink)) return apiLink; // already a direct image
  const url = new URL(apiLink);
  // card links end with a slash for the desktop client to append a language; without one Scryfall serves the
  // first image it has, which also covers promos without an English picture
  if (url.pathname.length > 1 && url.pathname.endsWith('/')) url.pathname = url.pathname.slice(0, -1);
  url.searchParams.set('format', 'image');
  url.searchParams.set('version', size);
  return url.toString();
}

function tokenLink(card: CardImageRef, set: string, links: ScryfallLinks): string | null {
  const imageNumber = card.imageNumber ?? 0;
  for (const name of new Set([tokenImageName(card), card.name ?? '', card.displayName ?? ''])) {
    if (!name) continue;
    const link = (imageNumber ? links.tokens[`${set}/${name}/${imageNumber}`] : undefined) ?? links.tokens[`${set}/${name}`];
    if (link) return link;
  }
  return null;
}

/** Link from the exported exception lists, if this card or token has one. */
export function exceptionLink(card: CardImageRef, face: CardFace, links: ScryfallLinks | null): string | null {
  if (!links) return null;
  const set = setCodeOf(card).toUpperCase();
  if (!set) return null;

  if (isTokenImage(card)) return tokenLink(card, set, links);

  const name = card.name ?? card.displayName ?? '';
  const number = (card.cardNumber ?? '').trim();
  if (name) {
    const faceSuffix = face === 'back' ? 'b' : '';
    const link = links.cards[`${set}/${name}/${number}${faceSuffix}`]
      ?? links.cards[`${set}/${name}/${number}`]
      ?? links.cards[`${set}/${name}`];
    if (link) return link;
  }
  // emblem, plane and dungeon views carry neither a type nor a collector number
  return number ? null : tokenLink(card, set, links);
}

/** Direct API image link for a printing; Scryfall redirects to its CDN. */
export function printingImageLink(card: CardImageRef, face: CardFace, size: ImageSize): string | null {
  const set = setCodeOf(card).toLowerCase();
  const number = normalizeCollectorNumber(card.cardNumber);
  if (!set || !number) return null;
  const url = new URL(`https://api.scryfall.com/cards/${encodeURIComponent(set)}/${encodeURIComponent(number)}`);
  url.searchParams.set('format', 'image');
  url.searchParams.set('version', size);
  if (face === 'back') url.searchParams.set('face', 'back');
  return url.toString();
}

/** Last resort: look the card up by name. */
export function namedImageLink(card: CardImageRef, face: CardFace, size: ImageSize): string | null {
  const name = card.name ?? card.displayName;
  if (!name) return null;
  const url = new URL('https://api.scryfall.com/cards/named');
  url.searchParams.set('exact', name);
  url.searchParams.set('format', 'image');
  url.searchParams.set('version', size);
  if (face === 'back') url.searchParams.set('face', 'back');
  return url.toString();
}

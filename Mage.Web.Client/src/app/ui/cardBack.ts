import { configuredImageProxy, proxiedScryfallUrl } from '../../core/images/imageProxy';

/**
 * Playmat's card backs (art/specs/cardback.json), one per design, in the order the picker shows them. They're painted
 * art, so with painted art off the printed CSS weave shows instead. Brown like a classic back, so a hidden card never
 * melts into the green cloth.
 */
export const CARD_BACKS = [
  'stitch-hand', 'stitched-rings', 'quiet', 'rosette', 'woven', 'sunburst', 'deck-stack', 'rings-five',
  'die-laurel', 'damask', 'world-tree', 'moon-peaks', 'labyrinth', 'orrery', 'knotwork', 'lantern',
] as const;

export type CardBackId = (typeof CARD_BACKS)[number] | 'classic';

export const DEFAULT_CARD_BACK: CardBackId = 'stitch-hand';

/** The original Magic card back as Scryfall serves it (its default card_back_id), through the image proxy if any. */
export const CLASSIC_BACK = proxiedScryfallUrl(
  'https://backs.scryfall.io/large/0/a/0aeebaf5-8c7d-4636-9e82-8c27447861f7.jpg',
  configuredImageProxy(),
);

/** The picture of a card back choice (whatever the painted-art setting). */
export function cardBackImage(id: CardBackId): string {
  if (id === 'classic') return CLASSIC_BACK;
  return `/card-back/${(CARD_BACKS as readonly string[]).includes(id) ? id : DEFAULT_CARD_BACK}.webp`;
}

/**
 * The picture on a hidden card: a sleeve's own back wins (a Career sleeve's painted one with painted art on, the
 * supporter sleeve's drawn one always), then the chosen back (the classic one always, a painted one only with painted
 * art on). Null means the printed weave.
 */
export function cardBackOf(sleeveBack: string | null, { paintedArt, back }: { paintedArt: boolean; back: CardBackId }): string | null {
  if (sleeveBack && (paintedArt || sleeveBack.startsWith('data:'))) return sleeveBack;
  if (back === 'classic') return CLASSIC_BACK;
  return paintedArt ? cardBackImage(back) : null;
}

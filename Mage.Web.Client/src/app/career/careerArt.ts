import { useSettings } from '../stores/settings';

/**
 * The painted art Career ships (unless the player turns it off in Settings, then the drawn SVG and CSS art shows): transparent WebP pictures from the art pipeline (art/generate.mjs and art/specs), one
 * folder per kind under public/career-art, every picture of a kind the same size. A kind's ids are its spec's items;
 * careerArt.test.ts checks every id here has its picture.
 */
export type ArtKind = 'achievement' | 'reward' | 'frame' | 'avatar' | 'portrait' | 'crest' | 'sleeve' | 'playmat' | 'companion' | 'sticker' | 'stamp';

export function artUrl(kind: ArtKind, id: string): string {
  return `/career-art/${kind}/${id}.webp`;
}

export type RewardArt =
  | 'coins' | 'xp' | 'wildcard-common' | 'wildcard-uncommon' | 'wildcard-rare' | 'wildcard-mythic' | 'pack' | 'title'
  | 'achievement' | 'level' | 'graduate' | 'unlock' | 'lock' | 'secret' | 'sleeve' | 'playmat';

/** The wildcard picture for a rarity (the server's rarity names). */
export function wildcardArt(rarity: string | undefined): RewardArt {
  const name = (rarity ?? '').toLowerCase();
  return name === 'common' || name === 'uncommon' || name === 'mythic' ? `wildcard-${name}` : 'wildcard-rare';
}

/** Whether painted art shows (Settings, Display: Painted art; on unless the player turned it off). */
export function usePaintedArt(): boolean {
  return useSettings((state) => state.settings.paintedArt);
}

/**
 * Showcase badges (B307): what a badge id names. The server gives one for every achievement earned (its id), every
 * tier of a tiered one ("t-spells-2") and every mastery rank ("mastery-color-w-5"); a player shows up to three.
 */

export const SHOWCASE_SIZE = 3;
export const TIER_NAMES = ['bronze', 'silver', 'gold', 'mythic'] as const;
export type TierName = (typeof TIER_NAMES)[number];

export type Badge =
  | { kind: 'achievement'; achievement: string; tier: number }
  | { kind: 'mastery'; track: string; rank: number };

export function badgeOf(id: string): Badge {
  const mastery = /^mastery-(color|format)-([a-z]+)-(\d+)$/.exec(id);
  if (mastery) {
    const value = mastery[2];
    const track = `${mastery[1]}_${mastery[1] === 'color' && value.length === 1 ? value.toUpperCase() : value}`;
    return { kind: 'mastery', track, rank: Number(mastery[3]) };
  }
  const tiered = /^(t-.+)-([1-4])$/.exec(id);
  if (tiered) return { kind: 'achievement', achievement: tiered[1], tier: Number(tiered[2]) };
  return { kind: 'achievement', achievement: id, tier: 0 };
}

/** A tier's name: 1 is bronze, 4 mythic; 0 (an achievement without tiers) has none. */
export function tierName(tier: number): TierName | null {
  return tier >= 1 && tier <= 4 ? TIER_NAMES[tier - 1] : null;
}

/** The badge id for an achievement's tier. */
export function badgeId(achievement: string, tier: number): string {
  return tier > 0 ? `${achievement}-${tier}` : achievement;
}

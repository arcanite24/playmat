import type { CardView } from '../../protocol/generated/views';

const COLOR_ORDER = ['white', 'blue', 'black', 'red', 'green'] as const;

/** White, blue, black, red, green, then multicolor, then colorless. */
function colorRank(card: CardView): number {
  const colors = COLOR_ORDER.filter((color) => card.color?.[color]);
  if (colors.length === 0) return COLOR_ORDER.length + 1;
  return colors.length > 1 ? COLOR_ORDER.length : COLOR_ORDER.indexOf(colors[0]);
}

/** The hand as Arena lays it out: lands first, then by mana value, then by color, then by name. */
export function sortHand<T extends CardView>(cards: readonly T[]): T[] {
  const isLand = (card: CardView) => (card.cardTypes ?? []).includes('LAND');
  return [...cards].sort((a, b) =>
    Number(isLand(b)) - Number(isLand(a))
    || (a.manaValue ?? 0) - (b.manaValue ?? 0)
    || colorRank(a) - colorRank(b)
    || (a.name ?? '').localeCompare(b.name ?? ''));
}

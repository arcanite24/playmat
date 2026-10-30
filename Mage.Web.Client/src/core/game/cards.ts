import type { CardView, StackAbilityView } from '../../protocol/generated/views';

/**
 * Whether a stack object is an ability rather than a spell. The server's ability views never set `isAbility`;
 * they carry the source card and an ABILITY_* object type instead.
 */
export function isStackAbility(card: CardView): card is StackAbilityView {
  return !!card.isAbility
    || !!(card as StackAbilityView).sourceCard
    || (card.mageObjectType ?? '').startsWith('ABILITY');
}

/**
 * Whether a card's text frame prints a power/toughness box. The server sends "0" for every card, so the type
 * decides: creatures and vehicles (by card type, else by the type line); with neither known, a box that isn't 0/0.
 */
export function printsPowerToughness(card: Pick<CardView, 'power' | 'toughness' | 'cardTypes' | 'subTypes'> & { typeText?: string }): boolean {
  if (!card.power || card.toughness === undefined) return false;
  if (card.cardTypes) return card.cardTypes.includes('CREATURE') || (card.subTypes ?? []).some((type) => type === 'VEHICLE' || type === 'SPACECRAFT');
  if (card.typeText) return /\b(?:creature|vehicle|spacecraft)\b/i.test(card.typeText);
  return card.power !== '0' || card.toughness !== '0';
}

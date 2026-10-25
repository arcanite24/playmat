import type { CardView, PermanentView } from '../../protocol/generated/views';
import { keywordsOf } from './glossary';

/**
 * The combat-relevant keywords a permanent has right now, for marks on the board: whether it flies, can block
 * fliers, needs two blockers, and so on. "Gained" means the printed card doesn't have it: an effect granted it, so the
 * picture alone won't tell the player.
 */
export interface KeywordMark {
  name: string;
  gained: boolean;
}

/** Board order: evasion first (it decides blocks), then combat damage, then protection. */
export const MARKED_KEYWORDS = [
  'Flying', 'Reach', 'Menace', 'Defender',
  'First strike', 'Double strike', 'Deathtouch', 'Trample', 'Lifelink', 'Infect', 'Vigilance', 'Haste',
  'Indestructible', 'Hexproof', 'Shroud', 'Ward', 'Protection',
  // not a keyword, but blocks depend on it (Fear of Being Hunted)
  'Must be blocked',
] as const;

export type MarkedKeyword = typeof MARKED_KEYWORDS[number];

/** An attacker that must be blocked if able, from its current rules text. */
export function mustBeBlocked(card: CardView | undefined): boolean {
  return (card?.rules ?? []).some((line) => /must be blocked\b/i.test(line.replace(/<[^>]+>/g, '')));
}

/** The server's ability icons (computed from the permanent's current abilities, granted ones included). */
const ICON_KEYWORDS: Record<string, MarkedKeyword> = {
  ABILITY_FLYING: 'Flying',
  ABILITY_REACH: 'Reach',
  ABILITY_DEFENDER: 'Defender',
  ABILITY_FIRST_STRIKE: 'First strike',
  ABILITY_DOUBLE_STRIKE: 'Double strike',
  ABILITY_DEATHTOUCH: 'Deathtouch',
  ABILITY_TRAMPLE: 'Trample',
  ABILITY_LIFELINK: 'Lifelink',
  ABILITY_INFECT: 'Infect',
  ABILITY_VIGILANCE: 'Vigilance',
  ABILITY_INDESTRUCTIBLE: 'Indestructible',
  ABILITY_HEXPROOF: 'Hexproof',
};

interface ServerIcon {
  cardIconType?: string;
}

export function keywordMarks(card: CardView): KeywordMark[] {
  const now = new Set<string>();
  for (const icon of (card.cardIcons ?? []) as ServerIcon[]) {
    const name = icon.cardIconType ? ICON_KEYWORDS[icon.cardIconType] : undefined;
    if (name) now.add(name);
  }
  // the server has no icon for these; its current rules text lists them, granted ones included
  for (const name of keywordsOf(card)) now.add(name);
  if (mustBeBlocked(card)) now.add('Must be blocked');
  const original = (card as PermanentView).original;
  const printed = original ? new Set(keywordsOf(original)) : null;
  return MARKED_KEYWORDS
    .filter((name) => now.has(name))
    .map((name) => ({ name, gained: !!printed && !printed.has(name) && !(card.isToken ?? false) }));
}

/**
 * What a permanent's keywords look like on the board, beyond their icons: a flier floats over its shadow, deathtouch
 * and infect smoke, trample raises dust, lifelink glows, first and double strike flash a blade, indestructible is rimmed
 * in gold, the hard-to-touch keywords hold a ward around it, menace smoulders and haste flickers. One effect per kind,
 * in the order the marks come in.
 */
export type KeywordEffect = 'float' | 'death' | 'infect' | 'dust' | 'life' | 'strike' | 'doubleStrike' | 'gold' | 'ward' | 'menace' | 'haste';

const EFFECTS: Partial<Record<MarkedKeyword, KeywordEffect>> = {
  Flying: 'float',
  Deathtouch: 'death',
  Infect: 'infect',
  Trample: 'dust',
  Lifelink: 'life',
  'First strike': 'strike',
  'Double strike': 'doubleStrike',
  Indestructible: 'gold',
  Hexproof: 'ward',
  Shroud: 'ward',
  Ward: 'ward',
  Protection: 'ward',
  Menace: 'menace',
  Haste: 'haste',
};

export function keywordEffects(marks: readonly KeywordMark[]): KeywordEffect[] {
  const effects = new Set<KeywordEffect>();
  for (const mark of marks) {
    const effect = EFFECTS[mark.name as MarkedKeyword];
    if (effect) effects.add(effect);
  }
  // double strike already flashes twice
  if (effects.has('doubleStrike')) effects.delete('strike');
  return [...effects];
}

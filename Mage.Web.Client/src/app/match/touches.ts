import type { CSSProperties } from 'react';
import type { CareerCatalogItem, PlayerLook } from '../../protocol/generated/views';
import { catalogItem } from '../ui/cosmeticsCatalog';
import { PAINTED_STYLE } from '../ui/landStyles';
import { clothOf, type MatCloth } from './playmats';

/**
 * The battlefield touches a player earned (B295, B300, B301), read from their look and the cosmetics catalog: the cloth
 * their half of the table is dyed in, the color of their attack arrows and hit sparks, the stamp pressed on the result
 * when they win and the set their basic lands are drawn from. Anything unknown (an id the catalog doesn't have yet, a
 * catalog still loading) gives nothing, and the table keeps its usual look.
 */

type Lookup = (kind: string, id: string | undefined) => CareerCatalogItem | undefined;

const fromCatalog: Lookup = (kind, id) => catalogItem(kind, id);

const HEX = /^#[0-9a-f]{3,8}$/i;

function colorOf(kind: 'arrow' | 'spark', id: string | undefined, lookup: Lookup): string | null {
  const color = id ? lookup(kind, id)?.color : undefined;
  return color && HEX.test(color) ? color : null;
}

/** A player's attack-arrow color, or null for the usual oxblood. */
export function arrowColorOf(look: PlayerLook | null | undefined, lookup: Lookup = fromCatalog): string | null {
  return colorOf('arrow', look?.arrow, lookup);
}

/** The color of a player's hit sparks, or null for none. */
export function sparkColorOf(look: PlayerLook | null | undefined, lookup: Lookup = fromCatalog): string | null {
  return colorOf('spark', look?.spark, lookup);
}

/** A player's victory stamp (its art is /career-art/stamp/<id>.webp), or null. */
export function stampOf(look: PlayerLook | null | undefined, lookup: Lookup = fromCatalog): string | null {
  const id = look?.stamp;
  return id && lookup('stamp', id) ? id : null;
}

export function stampArt(id: string): string {
  return `/career-art/stamp/${id}.webp`;
}

/** A player's land style as the styler takes it (ui/landStyles.ts): a set code, "painted:<style>", or undefined. */
export function landStyleKey(look: PlayerLook | null | undefined, lookup: Lookup = fromCatalog): string | undefined {
  const item = look?.lands ? lookup('land', look.lands) : undefined;
  if (!item) return undefined;
  return item.set ? item.set.toUpperCase() : `${PAINTED_STYLE}${item.id}`;
}

/**
 * The cloth a player chose, or null when they chose none or it isn't known here. A cloth of the stage's own list
 * (playmats.ts) is used as it is; a Career cloth only the catalog knows brings its own shades and dye.
 */
export function lookCloth(look: PlayerLook | null | undefined, lookup: Lookup = fromCatalog): MatCloth | null {
  const id = look?.cloth;
  if (!id) return null;
  const known = clothOf(id);
  if (known.id === id) return known;
  const item = lookup('playmat', id);
  const shades = item?.shades;
  if (!item?.dye || !shades || shades.length !== 7) return null;
  return { id, shades: shades as MatCloth['shades'], dye: item.dye, career: true };
}

/**
 * The box and dye of an opponent's cloth on their half of the table: the whole half (MatchStage.module.css
 * .theirCloth) for one opponent, their column of the battlefield for several. The cloth's own shades, darkest at the
 * edges like the stage's, so their cards read the same on it.
 */
export function theirClothBox(cloth: MatCloth, column: { left: number; width: number } | null): CSSProperties {
  const style: Record<string, string | number> = { '--their-950': cloth.shades[0], '--their-700': cloth.shades[3] };
  if (column) {
    style.left = column.left - 12;
    style.width = column.width + 24;
  }
  return style as CSSProperties;
}

/**
 * Each permanent's arrow color, for the players who earned one: `players` pairs each player's permanent ids with
 * their look. Every permanent is in it (not only attackers), so a stack's lead colors its arrow whichever member attacks.
 */
export function arrowTints(
  players: readonly { look: PlayerLook | null | undefined; permanents: Iterable<string> }[],
  lookup: Lookup = fromCatalog,
): Map<string, string> {
  const tints = new Map<string, string>();
  for (const player of players) {
    const color = arrowColorOf(player.look, lookup);
    if (!color) continue;
    for (const id of player.permanents) tints.set(id, color);
  }
  return tints;
}

/** "a=x,b=,c=y" (player id = value) back into a map of the non-empty values: lookups keyed by text stay stable. */
export function pairsOf(key: string): Map<string, string> {
  const pairs = new Map<string, string>();
  for (const pair of key.split(',')) {
    const at = pair.indexOf('=');
    if (at > 0 && at < pair.length - 1) pairs.set(pair.slice(0, at), pair.slice(at + 1));
  }
  return pairs;
}

import type { CSSProperties } from 'react';
import { catalogOf, onCatalog } from '../ui/cosmeticsCatalog';

/** The neoprene a player's mat is dyed in: the stage's shades, darkest to lightest, and the dye its art takes. */
export interface MatCloth {
  id: string;
  /** 950, 900, 800, 700, 600, 500, 400 */
  shades: [string, string, string, string, string, string, string];
  dye: string;
  /** unlocked in Career (a playmat cosmetic of the same id) */
  career?: boolean;
  /** a thank-you for supporting the server (a supporter perk of the same id) */
  perk?: boolean;
}

/** The free cloths everyone has. */
const FREE_CLOTHS: MatCloth[] = [
  { id: 'green', shades: ['#070c0a', '#0b120f', '#0f1714', '#142019', '#1c2d27', '#27403a', '#3a5a51'], dye: '#2f6b58' },
  { id: 'navy', shades: ['#070a10', '#0b1018', '#0f1520', '#141c2b', '#1c273a', '#28364f', '#3a4c6b'], dye: '#2f4c7a' },
  { id: 'plum', shades: ['#0d080d', '#140c14', '#1a101a', '#231523', '#301d30', '#432943', '#5c3b5c'], dye: '#6b3566' },
  { id: 'oxblood', shades: ['#0e0707', '#160b0b', '#1d0f0e', '#271413', '#361c1a', '#4b2724', '#663934'], dye: '#7a302a' },
  { id: 'teal', shades: ['#060c0d', '#091315', '#0d191b', '#112225', '#182f33', '#224247', '#335b61'], dye: '#2a6b70' },
  { id: 'charcoal', shades: ['#09090a', '#0e0e10', '#141416', '#1b1b1e', '#25252a', '#34343b', '#4a4a53'], dye: '#55555f' },
];

/** supporters' thanks: a deep verdigris with a brass dye, like the supporter sleeve */
const PATRON_CLOTH: MatCloth = { id: 'patron', shades: ['#050c0a', '#081410', '#0c1b16', '#11261f', '#18352c', '#224a3e', '#326656'], dye: '#a9813f', perk: true };

/**
 * Every cloth: the free ones, Career's (from the cosmetics catalog, ui/cosmeticsCatalog.ts: refilled in place when a
 * new catalog arrives) and the supporters' one.
 */
export const MAT_CLOTHS: MatCloth[] = [...FREE_CLOTHS, PATRON_CLOTH];

onCatalog((items) => {
  const career = catalogOf('playmat', items).flatMap((item): MatCloth[] => (item.id && item.dye && item.shades?.length === 7
    ? [{ id: item.id, shades: item.shades as MatCloth['shades'], dye: item.dye, career: true }]
    : []));
  MAT_CLOTHS.splice(0, MAT_CLOTHS.length, ...FREE_CLOTHS, ...career, PATRON_CLOTH);
});

export const DEFAULT_CLOTH = MAT_CLOTHS[0];

export function clothOf(id: string | null | undefined): MatCloth {
  return MAT_CLOTHS.find((cloth) => cloth.id === id) ?? DEFAULT_CLOTH;
}

/** A cloth's swatch: its light middle fading to its dark edge, rimmed in its dye. */
export function clothSwatch(cloth: MatCloth): CSSProperties {
  return { background: `radial-gradient(circle at 50% 40%, ${cloth.shades[4]}, ${cloth.shades[1]})`, borderColor: cloth.dye };
}

const SHADE_NAMES = ['950', '900', '800', '700', '600', '500', '400'];

/** The custom properties that re-dye the match stage (and the mat print) in a cloth. */
export function clothStyle(cloth: MatCloth): CSSProperties {
  const style: Record<string, string> = { '--print-dye': cloth.dye };
  cloth.shades.forEach((shade, index) => {
    style[`--mat-${SHADE_NAMES[index]}`] = shade;
  });
  return style as CSSProperties;
}

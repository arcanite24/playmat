import type { CareerCatalogItem } from '../../protocol/generated/views';

/**
 * Earned plate frames (B296) without React: how a catalog frame (its ring and accent colours and its ornament) is drawn
 * round an avatar. Kept apart from the rest of the wardrobe so the avatar discs everywhere can draw one cheaply.
 */

export type FrameStyle = 'solid' | 'double' | 'dashed' | 'beaded' | 'notched' | 'rays';
const FRAME_STYLES: FrameStyle[] = ['solid', 'double', 'dashed', 'beaded', 'notched', 'rays'];

export interface FrameLook {
  ring: string;
  accent: string;
  style: FrameStyle;
}

/** A frame as the catalog describes it; an unknown style draws solid, a missing accent is the ring lightened. */
export function frameLook(item: Pick<CareerCatalogItem, 'ring' | 'accent' | 'style'> | undefined): FrameLook | null {
  if (!item?.ring) return null;
  const style = FRAME_STYLES.includes(item.style as FrameStyle) ? (item.style as FrameStyle) : 'solid';
  return { ring: item.ring, accent: item.accent ?? lighten(item.ring, 0.45), style };
}

/** A #rrggbb colour moved toward white by an amount (0..1). */
export function lighten(hex: string, amount: number): string {
  const match = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!match) return hex;
  const value = parseInt(match[1], 16);
  const channel = (shift: number) => {
    const c = (value >> shift) & 255;
    return Math.round(c + (255 - c) * amount).toString(16).padStart(2, '0');
  };
  return `#${channel(16)}${channel(8)}${channel(0)}`;
}

/** One stroke or shape of a frame, in a 100 by 100 box whose middle 78 is the avatar. */
export type FrameShape =
  | { type: 'circle'; r: number; width: number; paint: 'ring' | 'accent'; dash?: string }
  | { type: 'dot'; x: number; y: number; r: number; paint: 'ring' | 'accent' }
  | { type: 'line'; x1: number; y1: number; x2: number; y2: number; width: number; paint: 'ring' | 'accent' }
  | { type: 'path'; d: string; paint: 'ring' | 'accent' };

const round = (value: number) => Math.round(value * 100) / 100;
const at = (angle: number, radius: number) => ({ x: round(50 + radius * Math.sin(angle)), y: round(50 - radius * Math.cos(angle)) });

/**
 * What a frame style draws round the avatar. Each reads apart from the supporters' thin brass ring: a heavy bezel, two
 * bands, stitches, beads on a thread, a band with points, or a sunburst.
 */
export function frameShapes(style: FrameStyle): FrameShape[] {
  const turn = (index: number, count: number) => (index / count) * Math.PI * 2;
  switch (style) {
    case 'double':
      return [
        { type: 'circle', r: 46, width: 2.6, paint: 'ring' },
        { type: 'circle', r: 41.5, width: 2.6, paint: 'ring' },
        { type: 'circle', r: 43.75, width: 0.8, paint: 'accent' },
      ];
    case 'dashed':
      return [
        { type: 'circle', r: 43.5, width: 4.5, paint: 'ring', dash: '6.8 4' },
        { type: 'circle', r: 40.2, width: 1, paint: 'accent' },
      ];
    case 'beaded':
      return [
        { type: 'circle', r: 43.5, width: 1.2, paint: 'ring' },
        ...Array.from({ length: 18 }, (_, index): FrameShape => ({ type: 'dot', ...at(turn(index, 18), 43.5), r: 3, paint: index % 3 === 0 ? 'accent' : 'ring' })),
      ];
    case 'notched':
      return [
        { type: 'circle', r: 42.5, width: 4, paint: 'ring' },
        ...Array.from({ length: 8 }, (_, index): FrameShape => {
          const angle = turn(index, 8);
          const tip = at(angle, 49.5);
          const left = at(angle - 0.13, 43);
          const right = at(angle + 0.13, 43);
          return { type: 'path', d: `M${left.x} ${left.y}L${tip.x} ${tip.y}L${right.x} ${right.y}Z`, paint: 'accent' };
        }),
      ];
    case 'rays':
      return [
        ...Array.from({ length: 24 }, (_, index): FrameShape => {
          const angle = turn(index, 24);
          const from = at(angle, 43);
          const to = at(angle, index % 2 === 0 ? 49.5 : 47);
          return { type: 'line', x1: from.x, y1: from.y, x2: to.x, y2: to.y, width: index % 2 === 0 ? 2 : 1.3, paint: 'accent' };
        }),
        { type: 'circle', r: 42, width: 3.2, paint: 'ring' },
      ];
    default:
      return [
        { type: 'circle', r: 43.5, width: 5.5, paint: 'ring' },
        { type: 'circle', r: 40.4, width: 1, paint: 'accent' },
        { type: 'circle', r: 46.6, width: 0.8, paint: 'accent' },
      ];
  }
}

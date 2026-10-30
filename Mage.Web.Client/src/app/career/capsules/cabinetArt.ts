/**
 * The painted cabinets (art/cabinet.mjs draws them, art/cabinet_cut.py cuts them): each machine's body with its globe
 * left empty, its crank wheel apart so it can turn, and one capsule shared by every machine, painted pale so it takes
 * a set's colour. The client puts the capsules into the globe and the wheel on its mount, so it needs to know where
 * they are: measured on the cut body, in its own pixels.
 */
export interface CabinetArt {
  body: string;
  crank: string;
  /** the cut body's size */
  width: number;
  height: number;
  /** the globe's glass: its centre and inner radius, and the floor the capsules rest on (the collar's top) */
  globe: { x: number; y: number; r: number; floor: number };
  /** the crank mount's centre, and the wheel's radius over it */
  mount: { x: number; y: number; r: number };
  /** the coin slot's top */
  slot: { x: number; y: number };
  /** the delivery chute's opening */
  chute: { left: number; right: number; top: number; bottom: number };
}

const CABINETS: Record<string, CabinetArt> = {
  'full-art': {
    body: '/career-art/cabinet/full-art.webp',
    crank: '/career-art/cabinet/full-art-crank.webp',
    width: 640,
    height: 1229,
    globe: { x: 318, y: 424, r: 292, floor: 600 },
    mount: { x: 459, y: 876, r: 109 },
    slot: { x: 158, y: 826 },
    chute: { left: 232, right: 408, top: 1012, bottom: 1110 },
  },
};

export function cabinetArtOf(machine: string | undefined): CabinetArt | undefined {
  return machine ? CABINETS[machine] : undefined;
}

/** The painted capsule: pale plastic on top (tinted to the set), frosted below; its seam's height, as a fraction. */
export const CAPSULE_ART = '/career-art/cabinet/capsule.webp';
export const CAPSULE_SEAM = 0.555;

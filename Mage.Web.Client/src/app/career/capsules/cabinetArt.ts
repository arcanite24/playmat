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
  'commander-vault': {
    body: '/career-art/cabinet/commander-vault.webp',
    crank: '/career-art/cabinet/commander-vault-crank.webp',
    width: 640,
    height: 1196,
    globe: { x: 318, y: 368, r: 268, floor: 556 },
    mount: { x: 460, y: 826, r: 98 },
    slot: { x: 154, y: 788 },
    chute: { left: 236, right: 400, top: 960, bottom: 1090 },
  },
  'planar-horizons': {
    body: '/career-art/cabinet/planar-horizons.webp',
    crank: '/career-art/cabinet/planar-horizons-crank.webp',
    width: 640,
    height: 1197,
    globe: { x: 320, y: 392, r: 272, floor: 592 },
    mount: { x: 462, y: 813, r: 102 },
    slot: { x: 167, y: 772 },
    chute: { left: 236, right: 404, top: 944, bottom: 1075 },
  },
  'tabletop-atelier': {
    body: '/career-art/cabinet/tabletop-atelier.webp',
    crank: '/career-art/cabinet/tabletop-atelier-crank.webp',
    width: 640,
    height: 1141,
    globe: { x: 320, y: 368, r: 266, floor: 556 },
    mount: { x: 458, y: 780, r: 98 },
    slot: { x: 162, y: 742 },
    chute: { left: 236, right: 404, top: 904, bottom: 1030 },
  },
  'legends-showcase': {
    body: '/career-art/cabinet/legends-showcase.webp',
    crank: '/career-art/cabinet/legends-showcase-crank.webp',
    width: 640,
    height: 1178,
    globe: { x: 320, y: 382, r: 270, floor: 576 },
    mount: { x: 460, y: 800, r: 100 },
    slot: { x: 164, y: 760 },
    chute: { left: 236, right: 404, top: 930, bottom: 1058 },
  },
};

export function cabinetArtOf(machine: string | undefined): CabinetArt | undefined {
  return machine ? CABINETS[machine] : undefined;
}

/** The painted capsule: pale plastic on top (tinted to the set), frosted below; its seam's height, as a fraction. */
export const CAPSULE_ART = '/career-art/cabinet/capsule.webp';
export const CAPSULE_SEAM = 0.555;

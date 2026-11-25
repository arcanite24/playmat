const GAP = 16;

/** Where the preview goes: beside the box it should sit outside of, else beside the card, never over the box to avoid. */
export function placePreview(anchor: DOMRect, span: number, viewport: number, avoid?: DOMRect, outside?: DOMRect): number {
  const right = (box: DOMRect) => box.right + GAP;
  const left = (box: DOMRect) => box.left - GAP - span;
  const fits = (x: number) => x >= GAP && x + span <= viewport - GAP && !(avoid && x < avoid.right && x + span > avoid.left);
  const candidates = [
    ...(outside ? [left(outside), right(outside)] : []),
    right(anchor),
    left(anchor),
  ];
  return candidates.find(fits) ?? Math.max(GAP, Math.min(viewport - GAP - span, left(anchor)));
}

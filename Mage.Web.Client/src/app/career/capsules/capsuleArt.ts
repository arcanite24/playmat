import type { CareerMachineSet } from '../../../protocol/generated/views';
import { cardStyleArt } from '../../ui/cardStyles';

/** A capsule set's reward sleeve, painted in the set's style (art/wavespeed.mjs draws it with the set). */
export function sleeveArtOf(set: CareerMachineSet): string | undefined {
  return set.id ? `/career-art/sleeve/set-${set.id}.webp` : undefined;
}

/** Picture URL for any capsule item: cardstyles use cardStyleArt, mixed-machine items use their cosmetic folder. */
export function capsuleItemArt(kind: string | undefined, id: string | undefined, card?: string): string | undefined {
  if (!id) return undefined;
  if (!kind || kind === 'cardstyle') return cardStyleArt(id, card ?? '');
  const dir = kind === 'pet' ? 'companion' : kind === 'frame' ? 'plate-frame' : kind;
  return `/career-art/${dir}/${id}.webp`;
}

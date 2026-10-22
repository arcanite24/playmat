import type { CareerMachineSet } from '../../../protocol/generated/views';

/** A capsule set's reward sleeve, painted in the set's style (art/machine-specs.mjs draws it with the set). */
export function sleeveArtOf(set: CareerMachineSet): string | undefined {
  return set.id ? `/career-art/sleeve/set-${set.id}.webp` : undefined;
}

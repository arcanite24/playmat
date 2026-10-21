import type { MessageKey } from '../i18n';

/**
 * Pets: a little 3D companion a player brings to the table, sitting by their side of the battlefield. Anyone at the
 * table can click one and it celebrates (only on that screen). Cosmetic only, like the rest of a player's look: the
 * server keeps a pet in the look only when the account owns it (Mage.Server support/Support.java validLook).
 *
 * Models are low-poly glTF from the art pipeline (Retro Diffusion's low-poly API), with named parts the scene animates:
 * a part missing from a model just doesn't move.
 */
export interface PetKind {
  /** the glTF binary, served from public/pets */
  model: string;
  /** a still portrait, served from public: the profile's pet shelf shows it (three.js loads only at the table) */
  still: string;
  name: MessageKey;
}

export const PETS: Record<string, PetKind> = {
  'party-corgi': { model: '/pets/party-corgi.glb', still: '/support/party-corgi-still.png', name: 'pet.party-corgi' },
};

export function petOf(id: string | undefined | null): PetKind | null {
  return id ? PETS[id] ?? null : null;
}

import type { MessageKey } from '../i18n';
import { catalogItem, catalogName } from '../ui/cosmeticsCatalog';

/**
 * Pets: a little 3D companion a player brings to the table, sitting by their side of the battlefield. Anyone at the
 * table can click one and it celebrates (only on that screen). Cosmetic only, like the rest of a player's look: the
 * server keeps a pet in the look only when the account owns it (Mage.Server support/Support.java validLook).
 *
 * Models are low-poly glTF from the art pipeline (Retro Diffusion's low-poly API), with named parts the scene animates:
 * a part missing from a model just doesn't move. Only the supporters' party corgi is 3D; the companions progression
 * gives (B297, pets in the cosmetics catalog) are painted stills that bob and hop in CSS (companionOf).
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

/** An earned companion: its painted still (art/specs/companion.json) and its name in the reader's language. */
export interface Companion {
  id: string;
  still: string;
  name: string;
}

export function companionOf(id: string | undefined | null, locale: string): Companion | null {
  if (!id || PETS[id]) return null;
  const item = catalogItem('pet', id);
  return item ? { id, still: `/career-art/companion/${id}.webp`, name: catalogName(item, locale) } : null;
}

//! Tamiyo's Compleation: "Flash. Enchant artifact, creature, or planeswalker. When Tamiyo's Compleation enters, tap
//! enchanted permanent. If it's an Equipment, unattach it. Enchanted permanent loses all abilities and doesn't untap
//! during its controller's untap step."
//!
//! (`TamiyosCompleationLoseAbilitiesEffect` is `LoseAllAbilitiesAttachedEffect`: transpile.py's `CUSTOM` table.)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TamiyosCompleation.TamiyosCompleationTapEffect", apply)];

/// the permanent the Aura is attached to (its last known one too) is tapped; an Equipment attached to a creature
/// is unattached
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (to, zcc) = game.obj(cx.source_id()).attached_to?;
    if game.obj(to).zone != Zone::Battlefield || game.obj(to).zcc != zcc {
        return Some(());
    }
    tap(game, to);
    if game.has_subtype(to, "Equipment") && game.attached(to).is_some() {
        game.obj_mut(to).attached_to = None;
        refresh_becomes(game);
    }
    Some(())
}

//! Watery Grasp: "Enchant creature. Enchanted creature doesn't untap during its controller's untap step. Waterbend
//! {5}: Enchanted creature's owner shuffles it into their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WateryGrasp.WateryGraspEffect", apply)];

/// The creature the Aura is (or was last) attached to, if it's still on the battlefield, shuffled into its owner's
/// library
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = game.obj(cx.source_id());
    let attached = if src.zone == Zone::Battlefield { src.attached_to } else { src.lki_attached_to };
    let Some((o, zcc)) = attached else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return Some(());
    }
    let owner = game.obj(o).owner;
    game.move_to(o, Zone::Library);
    let mut library = game.player(owner).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(owner).library = library;
    Some(())
}

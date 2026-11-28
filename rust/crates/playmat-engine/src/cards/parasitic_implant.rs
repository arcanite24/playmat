//! Parasitic Implant: "At the beginning of your upkeep, enchanted creature's controller sacrifices it and you create a
//! 1/1 colorless Phyrexian Myr artifact creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ParasiticImplant.ParasiticImplantEffect", apply)];

/// The Aura on the battlefield, attached to a permanent: that permanent is sacrificed
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let aura = cx.source_id();
    if game.obj(aura).zone != Zone::Battlefield {
        return Some(());
    }
    if let Some(o) = game.attached(aura) {
        sacrifice(game, o);
    }
    Some(())
}

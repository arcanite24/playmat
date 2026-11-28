//! Sage-Eye Avengers: "Prowess. Whenever Sage-Eye Avengers attacks, you may return target creature to its owner's
//! hand if its power is less than Sage-Eye Avengers's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SageEyeAvengers.SageEyeAvengersEffect", apply)];

/// the source's power as it is now (or as it last was)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let mine = game.power_toughness(cx.source_id()).0;
    if game.power_toughness(o).0 < mine {
        game.move_to(o, Zone::Hand);
    }
    Some(())
}

//! Blow Your House Down: "Up to three target creatures can't block this turn. Destroy any of them that are Walls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlowYourHouseDown.BlowYourHouseDownEffect", apply)];

/// Each legal target that's a Wall, in the order chosen, is destroyed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield && game.has_subtype(o, "Wall") {
            destroy(game, o);
        }
    }
    Some(())
}

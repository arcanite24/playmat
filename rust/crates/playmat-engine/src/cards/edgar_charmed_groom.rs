//! Edgar, Charmed Groom: "When Edgar, Charmed Groom dies, return it to the battlefield transformed under its owner's
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EdgarCharmedGroom.EdgarCharmedGroomEffect", apply)];

/// as Ojer Taq's, untapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::ojer_taq_deepest_foundation::return_transformed(game, d, cx, false)
}

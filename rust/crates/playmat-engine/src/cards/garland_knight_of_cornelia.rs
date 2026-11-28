//! Garland, Knight of Cornelia: "Whenever you cast a noncreature spell, surveil 1. {3}{B}{B}{R}{R}: Return this card
//! from your graveyard to the battlefield transformed. Activate only as a sorcery." Chaos, the Endless: "Flying. When
//! Chaos dies, put it on the bottom of its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GarlandKnightOfCornelia.GarlandKnightOfCorneliaEffect", apply)];

/// `getSourceObjectIfItStillExists`: the card still in the graveyard it was activated from, returned with its back
/// face up (`VALUE_KEY_ENTER_TRANSFORMED`) by `player.moveCards`, as Ojer Taq's untapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    super::ojer_taq_deepest_foundation::return_transformed(game, d, cx, false)
}

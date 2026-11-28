//! Karsus Depthguard: "Defender. As long as Karsus Depthguard's power is 5 or greater, it can attack as though it
//! didn't have defender."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("KarsusDepthguard.KarsusDepthguardCondition", power_five)];

/// Only while it's on the battlefield (`getSourcePermanentIfItStillExists`)
fn power_five(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.obj(source).zone == Zone::Battlefield && game.power_toughness(source).0 >= 5
}

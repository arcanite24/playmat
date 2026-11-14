//! Technodrome: "Reach, trample. This creature can't attack or block unless its power is 6 or greater. {T}, Sacrifice
//! another artifact: Draw a card. Put a +1/+1 counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("Technodrome.TechnodromeCondition", power_six)];

/// Its power, or as it last was on the battlefield (`getSourcePermanentOrLKI`)
fn power_six(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let power = if o.zone == Zone::Battlefield { Some(game.power_toughness(source).0) } else { o.power_lki };
    power.unwrap_or(0) >= 6
}

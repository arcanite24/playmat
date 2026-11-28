//! Viv Vision, Teen Synthezoid: "Flying. Cybernetic Senses — Whenever Viv Vision attacks, draw a card if her power is 4
//! or greater. Power-up — {7}: Put two +1/+1 counters on Viv Vision."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("VivVisionTeenSynthezoid.VivVisionTeenSynthezoidCondition", power_four)];

/// Its power, or as it last was on the battlefield (`getSourcePermanentOrLKI`)
fn power_four(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let power = if o.zone == Zone::Battlefield { Some(game.power_toughness(source).0) } else { o.power_lki };
    power.unwrap_or(0) >= 4
}

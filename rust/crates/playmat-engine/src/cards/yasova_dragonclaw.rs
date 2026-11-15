//! Yasova Dragonclaw: "Trample. At the beginning of combat on your turn, you may pay {1}{U/R}{U/R}. If you do, gain
//! control of target creature an opponent controls with power less than Yasova Dragonclaw's power until end of turn,
//! untap that creature, and it gains haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("YasovaDragonclaw.YasovaDragonclawPowerLessThanSourcePredicate", less_power_than_source)];

/// Less power than the source, as it is or last was on the battlefield (`getSourcePermanentOrLKI`)
fn less_power_than_source(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let o = game.obj(src);
    let power = if o.zone == Zone::Battlefield { Some(game.power_toughness(src).0) } else { o.power_lki };
    power.is_some_and(|p| game.power_toughness(id).0 < p)
}

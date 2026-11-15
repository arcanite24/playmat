//! Firja's Retribution: "I — Create a 4/4 white Angel Warrior creature token with flying and vigilance. II — Until
//! end of turn, Angels you control gain "{T}: Destroy target creature with less power than this creature." III —
//! Angels you control gain double strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("FirjasRetribution.FirjasRetributionPredicate", less_power_than_source)];

/// Less power than the source, as it is or last was on the battlefield (`getSourcePermanentOrLKI`)
fn less_power_than_source(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let o = game.obj(src);
    let power = if o.zone == Zone::Battlefield { Some(game.power_toughness(src).0) } else { o.power_lki };
    power.is_some_and(|p| game.power_toughness(id).0 < p)
}

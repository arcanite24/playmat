//! Alesha, Who Laughs at Fate: "Raid — At the beginning of your end step, if you attacked this turn, return target
//! creature card with mana value less than or equal to Alesha's power from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("AleshaWhoLaughsAtFate.AleshaWhoLaughsAtFatePredicate", at_most_alesha_power)];

/// Its mana value is at most the source's power (`getSourcePermanentOrLKI`); false with no source
fn at_most_alesha_power(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let power = if game.obj(src).zone == Zone::Battlefield {
        game.power_toughness(src).0
    } else {
        match game.obj(src).power_lki {
            Some(p) => p,
            None => return false,
        }
    };
    (game.obj(id).def.mana_value() as i32) <= power
}

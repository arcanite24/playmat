//! Narset, Enlightened Exile: "Whenever Narset attacks, exile target noncreature, nonland card with mana value less
//! than Narset's power from a graveyard and copy it. You may cast the copy without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("NarsetEnlightenedExile.NarsetEnlightenedExilePredicate", less_than_power)];

/// its mana value is less than the source's power (on the battlefield, else as it last was there)
fn less_than_power(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(s) = source else { return false };
    let power = if game.obj(s).zone == Zone::Battlefield {
        game.power_toughness(s).0
    } else {
        let Some(p) = game.obj(s).power_lki else { return false };
        p
    };
    (game.obj(id).def.mana_value() as i32) < power
}

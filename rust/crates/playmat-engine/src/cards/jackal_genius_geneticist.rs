//! Jackal, Genius Geneticist: "Whenever you cast a creature spell with mana value equal to Jackal's power, copy that
//! spell, except the copy isn't legendary. Then put a +1/+1 counter on Jackal."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("JackalGeniusGeneticist.JackalGeniusGeneticistPredicate", equal_to_power)];

/// the spell's mana value against the source's power (as it last was on the battlefield)
fn equal_to_power(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    game.obj(id).def.mana_value() as i32 == game.power_toughness(src).0
}

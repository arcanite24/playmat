//! Omnipresence: "You may cast spells with mana value less than or equal to the number of creatures you control from
//! your hand without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("Omnipresence.ManaValueLessThanControlledCreatureCountPredicate", few)];

fn few(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let creatures =
        game.battlefield.iter().filter(|p| game.obj(**p).controller == you && game.has_type(**p, CardType::Creature)).count();
    game.obj(id).def.mana_value() as usize <= creatures
}

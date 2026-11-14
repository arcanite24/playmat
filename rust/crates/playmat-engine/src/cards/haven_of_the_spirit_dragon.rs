//! Haven of the Spirit Dragon: "{2}, {T}, Sacrifice Haven of the Spirit Dragon: Return target Dragon creature card or
//! Ugin planeswalker card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("HavenOfTheSpiritDragon.DragonCreatureCardPredicate", dragon_creature),
    Handler::predicate("HavenOfTheSpiritDragon.UginPlaneswalkerCardPredicate", ugin_planeswalker),
];

fn dragon_creature(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.has_type(id, CardType::Creature) && game.has_subtype(id, "Dragon")
}

fn ugin_planeswalker(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.has_type(id, CardType::Planeswalker) && game.has_subtype(id, "Ugin")
}

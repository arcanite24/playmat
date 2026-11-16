//! Galvanic Bombardment: "Galvanic Bombardment deals X damage to target creature, where X is 2 plus the number of
//! cards named Galvanic Bombardment in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value(
    "GalvanicBombardment.GalvanicBombardmentCardsInControllerGraveyardCount",
    two_plus_named,
)];

/// 2 plus the cards named Galvanic Bombardment in your graveyard (the card's only filter, `NamePredicate`)
fn two_plus_named(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let named = game.player(you).graveyard.iter().filter(|c| game.obj(**c).name() == "Galvanic Bombardment").count();
    2 + named as i32
}

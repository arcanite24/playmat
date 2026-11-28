//! Go-Shintai of Hidden Cruelty: "Deathtouch. At the beginning of your end step, you may pay {1}. When you do,
//! destroy target creature with toughness X or less, where X is the number of Shrines you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("GoShintaiOfHiddenCruelty.GoShintaiOfHiddenCrueltyPredicate", toughness_at_most_shrines)];

/// its toughness is at most the number of Shrines the player choosing controls
fn toughness_at_most_shrines(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let shrines =
        game.battlefield.iter().filter(|p| game.obj(**p).controller == you && game.has_subtype(**p, "Shrine")).count();
    game.power_toughness(id).1 <= shrines as i32
}

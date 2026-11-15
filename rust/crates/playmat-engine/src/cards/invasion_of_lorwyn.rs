//! Invasion of Lorwyn: "When Invasion of Lorwyn enters, destroy target non-Elf creature an opponent controls with
//! power X or less, where X is the number of lands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("InvasionOfLorwyn.InvasionOfLorwynPredicate", power_at_most_lands)];

/// its power is at most the lands the source's controller controls
fn power_at_most_lands(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(source) = source else { return false };
    let you = game.obj(source).controller;
    let lands = game
        .battlefield
        .iter()
        .filter(|l| game.obj(**l).controller == you && game.has_type(**l, CardType::Land))
        .count() as i32;
    game.power_toughness(id).0 <= lands
}

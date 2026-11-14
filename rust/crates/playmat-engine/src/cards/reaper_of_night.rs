//! Reaper of Night: "Whenever Reaper of Night attacks, if defending player has two or fewer cards in hand, it gains
//! flying until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ReaperOfNight.ReaperOfNightCondition", few_cards)];

/// the player it's attacking (or whose planeswalker it is)
fn few_cards(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.obj(source).attacking.is_some_and(|p| game.player(p).hand.len() <= 2)
}

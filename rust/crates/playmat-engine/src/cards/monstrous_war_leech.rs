//! Monstrous War-Leech: "Kicker {U}. As Monstrous War-Leech enters, if it was kicked, mill four cards. Monstrous
//! War-Leech's power and toughness are each equal to the highest mana value among cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("MonstrousWarLeech.MonstrousWarLeechValue", highest_mv)];

/// the highest mana value among the cards in your graveyard, 0 with none
fn highest_mv(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.player(you).graveyard.iter().map(|c| game.obj(*c).def.mana_value() as i32).max().unwrap_or(0)
}

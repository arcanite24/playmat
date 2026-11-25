//! Storrev, Devkarin Lich: "Whenever Storrev deals combat damage to a player or planeswalker, return to your hand
//! target creature or planeswalker card in your graveyard that wasn't put there this combat."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("StorrevDevkarinLich.StorrevDevkarinLichPredicate", not_this_combat)];

/// not among the creature and planeswalker cards put into a graveyard since this turn's last beginning of combat
/// (its watcher's `MageObjectReference`s: the card as it is there)
fn not_this_combat(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let zcc = game.obj(id).zcc;
    !game.watch.to_graveyard_this_combat.contains(&(id, zcc))
}

//! Cruel Calculations: "Draw X cards, where X is the number of cards that were put into target player's graveyard
//! from their library this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("CruelCalculations.CruelCalculationsValue", milled_this_turn)];

/// `CruelCalculationsWatcher.getCount(getFirstTarget())`: the cards put into the target player's graveyard from a
/// library this turn
fn milled_this_turn(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let item = game.resolving_item.as_ref();
    let Some(Target::Player(p)) = item.and_then(|i| i.targets.first()).and_then(|g| g.first()).copied() else {
        return 0;
    };
    game.watch.library_to_graveyard.iter().filter(|s| **s == p).count() as i32
}

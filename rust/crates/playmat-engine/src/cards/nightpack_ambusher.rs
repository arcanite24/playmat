//! Nightpack Ambusher: "Flash. Other Wolves and Werewolves you control get +1/+1. At the beginning of your end step,
//! if you didn't cast a spell this turn, create a 2/2 green Wolf creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("NightpackAmbusher.NightpackAmbusherCondition", none_cast)];

/// `CastSpellLastTurnWatcher`: you cast no spell this turn
fn none_cast(game: &Game, you: Seat, _: ObjId) -> bool {
    !game.watch.spells.iter().any(|s| s.caster == you)
}

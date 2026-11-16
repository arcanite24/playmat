//! Rith, Liberated Primeval: "Flying, ward {2}. Other Dragons you control have ward {2}. At the beginning of your end
//! step, if a creature or planeswalker an opponent controlled was dealt excess damage this turn, create a 4/4 red
//! Dragon creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("RithLiberatedPrimeval.RithLiberatedPrimevalCondition", excess_to_opponents)];

/// `RithLiberatedPrimevalWatcher`: a permanent dealt excess damage this turn whose controller then is your opponent
/// (as it left the battlefield, if it did; else as it is now)
fn excess_to_opponents(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.damage.iter().any(|d| {
        let Target::Obj(o) = d.to else { return false };
        let controller = game
            .watch
            .left_battlefield
            .iter()
            .find(|l| l.id == o && l.zcc == d.to_zcc)
            .map_or(game.obj(o).controller, |l| l.controller);
        d.excess > 0 && controller != you
    })
}

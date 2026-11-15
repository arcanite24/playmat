//! The Tarrasque: "The Tarrasque has haste and ward {10} as long as it was cast. Whenever The Tarrasque attacks, it
//! fights target creature defending player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("TheTarrasque.TheTarrasqueCondition", was_cast)];

/// `TheTarrasqueWatcher`: the permanent is the card of a spell that was cast (it entered by resolving as a spell)
fn was_cast(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Battlefield && o.cast
}

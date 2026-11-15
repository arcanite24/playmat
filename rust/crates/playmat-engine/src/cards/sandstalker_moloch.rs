//! Sandstalker Moloch: "Flash. When this creature enters, if an opponent cast a blue and/or black spell this turn,
//! look at the top four cards of your library. You may reveal a permanent card from among them and put it into your
//! hand. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SandstalkerMoloch.SandstalkerMolochCondition", opponent_cast)];

/// `SandstalkerMolochWatcher`: a spell an opponent of yours cast this turn was blue or black (the card's colors)
fn opponent_cast(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().any(|s| {
        s.caster != you && game.colors(s.card).iter().any(|c| matches!(c, crate::card::Color::U | crate::card::Color::B))
    })
}

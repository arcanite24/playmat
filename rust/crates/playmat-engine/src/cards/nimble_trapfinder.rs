//! Nimble Trapfinder: "This creature can't be blocked if you had another Cleric, Rogue, Warrior, or Wizard enter
//! the battlefield under your control this turn. At the beginning of combat on your turn, if you have a full party,
//! creatures you control gain "Whenever this creature deals combat damage to a player, draw a card" until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("NimbleTrapfinder.NimbleTrapfinderCondition", another_entered)];

/// `NimbleTrapfinderWatcher.checkPlayer`: a Cleric, Rogue, Warrior or Wizard creature entered under the source's
/// controller's control this turn, other than the source (any, when the source isn't on the battlefield)
fn another_entered(game: &Game, _: Seat, source: ObjId) -> bool {
    let s = game.obj(source);
    let on = s.zone == Zone::Battlefield;
    let who = if on { s.controller } else { s.owner };
    game.watch.entered_permanents.iter().any(|(o, z, c)| {
        *c == who
            && !(on && *o == source && *z == s.zcc)
            && game.has_type(*o, CardType::Creature)
            && ["Cleric", "Rogue", "Warrior", "Wizard"].iter().any(|t| game.has_subtype(*o, t))
    })
}

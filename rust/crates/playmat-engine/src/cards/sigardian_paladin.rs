//! Sigardian Paladin: "As long as you've put one or more +1/+1 counters on a creature this turn, this creature has
//! trample and lifelink. {1}{G}{W}: Target creature you control with a +1/+1 counter on it gains trample and lifelink
//! until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SigardianPaladin.SigardianPaladinCondition", you_put_one)];

/// `SigardianPaladinWatcher`: you put a +1/+1 counter on a creature this turn
fn you_put_one(game: &Game, you: Seat, _: ObjId) -> bool {
    game.p1p1_put_this_turn.iter().any(|(p, o, _)| *p == you && game.has_type(*o, CardType::Creature))
}

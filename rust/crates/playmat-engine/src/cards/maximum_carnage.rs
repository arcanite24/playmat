//! Maximum Carnage: "I — Until your next turn, each creature attacks each combat if able and attacks a player other
//! than you if able. II — Add {R}{R}{R}. III — This Saga deals 5 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "MaximumCarnage.MaximumCarnageEffect",
    opponents_creature,
    &[Restrict::CantAttack(never)],
)
.until_your_next_turn()];

/// a permanent of a player whose opponent controls the Saga's ability
fn opponents_creature(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    game.obj(permanent).controller != cx.controller
}

/// `canAttack(attacker, defender)`: with two players left any attack is allowed, so nothing is stopped. (Choosing the
/// defender isn't a restriction the engine reads; with more players Java's "a player other than you if able" isn't
/// kept.)
fn never(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    false
}

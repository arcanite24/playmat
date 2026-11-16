//! `CantAttackUnlessDefenderControllsPermanent(filter)`: "{this} can't attack unless defending player controls an
//! Island" (Sealock Monster, Godhunter Octopus). Two players: the defending player is the attacker's opponent.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "CantAttackUnlessDefenderControllsPermanent",
    applies,
    &[Restrict::CantAttack(no_attack)],
)
.args(&["filter"])];

fn applies(_: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
}

/// `countAll(filter, defendingPlayerId)`: no permanent of the filter the defending player controls
fn no_attack(game: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    let defender = game.opponent(game.obj(attacker).controller);
    !game.battlefield.iter().any(|c| game.obj(*c).controller == defender && cx.matches(game, 0, *c))
}

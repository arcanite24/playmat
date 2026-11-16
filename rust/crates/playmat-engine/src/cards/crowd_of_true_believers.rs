//! Crowd of True Believers: "{T}: Target creature you control that's attacking alone gets +1/+0 until end of turn.
//! You gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("CrowdOfTrueBelievers.AttackingAlonePredicate", attacking_alone)];

/// It's attacking and it's the only attacker (`Combat.attacksAlone`: one group with one attacker)
fn attacking_alone(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).attacking.is_some() && attackers(game) == [id]
}

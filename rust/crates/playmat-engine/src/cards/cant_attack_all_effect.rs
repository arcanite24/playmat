//! `CantAttackAllEffect(Duration.WhileOnBattlefield, filter)`: "creatures without flying or islandwalk can't
//! attack" (Stormtide Leviathan)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "CantAttackAllEffect",
    applies,
    &[Restrict::CantAttack(no_attack)],
)
.args(&["while_on_battlefield", "filter"])];

fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    cx.matches(game, 0, permanent)
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

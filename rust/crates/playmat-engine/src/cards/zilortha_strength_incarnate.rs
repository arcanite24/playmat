//! Zilortha, Strength Incarnate: "Lethal damage dealt to creatures you control is determined by their power rather
//! than their toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "ZilorthaStrengthIncarnate.ZilorthaStrengthIncarnateEffect",
    Rule::LethalByPower(yours),
)];

/// `addPowerInsteadOfToughnessForDamageLethalityFilter(source, creatures with ControllerIdPredicate(you))`, active
/// while Zilortha is on the battlefield
fn yours(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.obj(id).controller == cx.controller && game.has_type(id, CardType::Creature)
}

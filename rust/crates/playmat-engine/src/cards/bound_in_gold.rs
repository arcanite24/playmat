//! Bound in Gold: "Enchant permanent. Enchanted permanent can't attack, block, or crew Vehicles, and its activated
//! abilities can't be activated unless they're mana abilities."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "BoundInGold.BoundInGoldEffect",
    Rule::Several(&[Rule::CantCrew(enchanted), Rule::CantActivate(of_enchanted)]),
)];

/// `CREW_VEHICLE` of the enchanted permanent
fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

/// `ACTIVATE_ABILITY` of an activated ability (not a mana ability) of the enchanted permanent
fn of_enchanted(game: &Game, cx: &StaticCx, _seat: Seat, id: ObjId, _index: usize) -> bool {
    game.attached(cx.source) == Some(id)
}

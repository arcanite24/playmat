//! Intercessor's Arrest: "Enchant permanent. Enchanted permanent can't attack or block, or crew Vehicles. Its
//! activated abilities can't be activated unless they're mana abilities."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "IntercessorsArrest.IntercessorsArrestEffect",
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

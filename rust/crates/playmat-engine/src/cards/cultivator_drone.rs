//! Cultivator Drone: "Devoid. {T}: Add {C}. Spend this mana only to cast a colorless spell, activate an ability of a
//! colorless permanent, or pay a cost that contains {C}."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("CultivatorDrone.CultivatorDroneManaBuilder", spend)];

/// a colorless spell, an ability of a colorless permanent, or a cost with {C} in it (the spell's mana cost, or the
/// ability's)
fn spend(game: &Game, cx: &SpendCx) -> bool {
    if game.colors(cx.on).is_empty() {
        return true;
    }
    if cx.spell() {
        return game.obj(cx.on).def.cost.colorless > 0;
    }
    cx.ability_def(game).and_then(|a| a.activated()).is_some_and(|(_, _, _, costs, _)| ability_mana_cost(costs).colorless > 0)
}

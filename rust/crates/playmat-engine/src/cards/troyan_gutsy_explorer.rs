//! Troyan, Gutsy Explorer: "{T}: Add {G}{U}. Spend this mana only to cast spells with mana value 5 or greater or
//! spells with {X} in their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("TroyanGutsyExplorer.TroyanGutsyExplorerManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    let def = &game.obj(cx.on).def;
    cx.spell() && (def.mana_value() >= 5 || def.cost.x > 0)
}

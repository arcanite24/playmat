//! Ixalli's Lorekeeper: "{T}: Add one mana of any color. Spend this mana only to cast a Dinosaur spell or activate an
//! ability of a Dinosaur source."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("IxallisLorekeeper.IxallisLorekeeperManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.has_subtype(cx.on, "Dinosaur")
}

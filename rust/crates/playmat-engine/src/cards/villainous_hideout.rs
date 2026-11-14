//! Villainous Hideout: "{T}: Add one mana of any color. Spend this mana only to cast a Villain spell or to activate
//! an ability of a Villain source."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::spend("VillainousHideout.VillainousHideoutManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    game.has_subtype(cx.on, "Villain")
}

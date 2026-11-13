//! Dragon Tempest: "Whenever a Dragon you control enters, it deals X damage to any target, where X is the number of
//! Dragons you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DragonTempest.DragonTempestDamageEffect", apply)];

/// The damage's source is the Dragon that entered (`getValue("permanentEnteringBattlefield")`), as it last was
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(dragon) = cx.item.entering else { return Some(()) };
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Dragon"))
        .count() as i32;
    if n > 0 {
        if let Some(t) = cx.target(0) {
            deal_damage(game, dragon, t, n, false);
        }
    }
    Some(())
}

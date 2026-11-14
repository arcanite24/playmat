//! Budding Insurgent: "Sacrifice this creature: Destroy target artifact or enchantment. If that permanent was a
//! legendary enchantment, draw a card. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BuddingInsurgent.BuddingInsurgentEffect", apply)];

/// destroyed (it can't be regenerated), then a draw if it was a legendary enchantment as it last was
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let legendary_enchantment = game.obj(o).def.supertypes.iter().any(|s| s == "LEGENDARY")
        && game.has_type(o, CardType::Enchantment);
    destroy_with(game, o, false);
    if legendary_enchantment {
        draw(game, cx.controller(), 1);
    }
    Some(())
}

//! Goblin Plate Mail: "When this Equipment enters, amass Goblins 1, then attach this Equipment to the Army you
//! amassed."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GoblinPlateMail.GoblinPlateMailEffect", apply)
    .effects(&["new AmassEffect(1, SubType.GOBLIN)"])];

/// The amass (as Widespread Brutality's), then this Equipment, while it's still the permanent that triggered,
/// attached to that Army
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(army) = super::widespread_brutality::amass(game, d, cx, 1)? else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(army).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(army).zcc;
    game.attach_to(cx.source_id(), army, zcc);
    Some(())
}

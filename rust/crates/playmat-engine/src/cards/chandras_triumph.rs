//! Chandra's Triumph: "Chandra's Triumph deals 3 damage to target creature or planeswalker an opponent controls.
//! Chandra's Triumph deals 5 damage to that permanent instead if you control a Chandra planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChandrasTriumph.ChandrasTriumphEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let chandra = game.battlefield.iter().any(|o| {
        game.obj(*o).controller == you && game.has_type(*o, CardType::Planeswalker) && game.has_subtype(*o, "Chandra")
    });
    deal_damage(game, cx.source_id(), Target::Obj(t), if chandra { 5 } else { 3 }, false);
    Some(())
}

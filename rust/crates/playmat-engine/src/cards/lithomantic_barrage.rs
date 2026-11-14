//! Lithomantic Barrage: "Lithomantic Barrage deals 1 damage to target creature or planeswalker. It deals 5 damage
//! instead if that target is white and/or blue."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LithomanticBarrage.LithomanticBarrageEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let colors = game.colors(o);
    let n = if colors.contains(&Color::W) || colors.contains(&Color::U) { 5 } else { 1 };
    deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
    Some(())
}

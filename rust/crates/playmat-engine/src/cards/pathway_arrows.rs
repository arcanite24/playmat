//! Pathway Arrows: "Equipped creature has \"{2}, {T}: This creature deals 1 damage to target creature. If a
//! colorless creature is dealt damage this way, tap it.\""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PathwayArrows.PathwayArrowsEffect", apply)];

/// the damage from the ability's source (the equipped creature), then the tap when some was dealt
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let dealt = deal_damage(game, cx.source_id(), Target::Obj(o), 1, false);
    if dealt > 0 && game.obj(o).zone == Zone::Battlefield && game.colors(o).is_empty() {
        tap(game, o);
    }
    Some(())
}

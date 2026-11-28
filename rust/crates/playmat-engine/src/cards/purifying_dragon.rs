//! Purifying Dragon: "Whenever Purifying Dragon attacks, it deals 1 damage to target creature defending player
//! controls. If that creature is a Zombie, Purifying Dragon deals 2 damage to that creature instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PurifyingDragon.PurifyingDragonEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let n = if game.has_subtype(o, "Zombie") { 2 } else { 1 };
    deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
    Some(())
}

//! Rending Flame: "Rending Flame deals 5 damage to target creature or planeswalker. If that permanent is a Spirit,
//! Rending Flame also deals 2 damage to that permanent's controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RendingFlame.RendingFlameEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    deal_damage(game, cx.source_id(), Target::Obj(t), 5, false);
    // Java reads the permanent object after the damage: a Spirit that died still counts, its controller as it was
    if game.has_subtype(t, "Spirit") {
        let p = game.obj(t).controller;
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    Some(())
}

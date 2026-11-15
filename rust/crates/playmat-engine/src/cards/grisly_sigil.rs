//! Grisly Sigil: "Casualty 1. Choose target creature or planeswalker. If it was dealt noncombat damage this turn,
//! Grisly Sigil deals 3 damage to it and you gain 3 life. Otherwise, Grisly Sigil deals 1 damage to it and you gain
//! 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GrislySigil.GrislySigilEffect", apply)];

/// `GrislySigilWatcher`: the target (as the object it is now) was dealt noncombat damage this turn: 3, else 1
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(o).zcc;
    let dealt = game.watch.damage.iter().any(|d| d.to == Target::Obj(o) && d.to_zcc == zcc && !d.combat);
    let amount = if dealt { 3 } else { 1 };
    deal_damage(game, cx.source_id(), Target::Obj(o), amount, false);
    gain_life(game, cx.controller(), amount);
    Some(())
}

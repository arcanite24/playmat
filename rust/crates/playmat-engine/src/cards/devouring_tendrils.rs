//! Devouring Tendrils: "Target creature you control deals damage equal to its power to target creature or
//! planeswalker you don't control. When the permanent you don't control dies this turn, you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DevouringTendrils.DevouringTendrilsEffect", apply).effects(&["new GainLifeEffect(2)"])];

/// A delayed "when it dies this turn" trigger armed on the second target as it is now
/// (`DevouringTendrilsDelayedTriggeredAbility`: the engine's "when it dies this turn", armed on that target)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(1) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let delayed = Delayed {
        rule: "When the permanent you don't control dies this turn, you gain 2 life.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let zcc = game.obj(o).zcc;
    let turn = game.turn;
    let t = StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) };
    game.when_dies.push((o, zcc, turn, false, t));
    Some(())
}

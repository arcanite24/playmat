//! Outmuscle: "Put a +1/+1 counter on target creature you control, then it fights target creature you don't control.
//! Adamant — If at least three green mana was spent to cast this spell, the creature you control gains
//! indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Outmuscle.OutmuscleEffect", apply)
    .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)"])];

/// adamant's indestructible first, then the counter, then (with the second target still there) the fight
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mine = cx.obj(0)?;
    if game.obj(mine).zone != Zone::Battlefield {
        return Some(());
    }
    if game.obj(cx.source_id()).spent_colors[4] >= 3 {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(mine)]);
    }
    put_counters(game, mine, "+1/+1", 1, cx.controller());
    if cx.obj(1).is_none() {
        return Some(());
    }
    cx.run(game, d, &[EffectDef::FightTargetsEffect]);
    (!d.stopped()).then_some(())
}

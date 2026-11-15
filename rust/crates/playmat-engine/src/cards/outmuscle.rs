//! Outmuscle: "Put a +1/+1 counter on target creature you control, then it fights target creature you don't control.
//! Adamant — If at least three green mana was spent to cast this spell, the creature you control gains
//! indestructible until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Outmuscle.OutmuscleEffect", apply)
    .effects(&["new GainAbilityTargetEffect(IndestructibleAbility.getInstance(), Duration.EndOfTurn)"])];

/// adamant's indestructible first, then the counter, then (with the second target still there) the fight, which
/// Java has the other creature start (`creature.fight(permanent)`): it's dealt all of your creature's power, and
/// yours only lethal damage (`fightWithExcess`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mine = cx.obj(0)?;
    if game.obj(mine).zone != Zone::Battlefield {
        return Some(());
    }
    if game.obj(cx.source_id()).spent_colors[4] >= 3 {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(mine)]);
    }
    put_counters(game, mine, "+1/+1", 1, cx.controller());
    let Some(theirs) = cx.obj(1) else { return Some(()) };
    if game.obj(theirs).zone != Zone::Battlefield {
        return Some(());
    }
    let p_mine = game.power_toughness(mine).0;
    let p_theirs = game.power_toughness(theirs).0;
    deal_damage(game, mine, Target::Obj(theirs), p_mine, false);
    let p_theirs = p_theirs.min(fight_lethal(game, mine, theirs));
    deal_damage(game, theirs, Target::Obj(mine), p_theirs, false);
    (!d.stopped()).then_some(())
}

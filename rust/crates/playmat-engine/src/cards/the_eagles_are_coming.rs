//! The Eagles Are Coming!: "Choose target creature you own. If this spell was kicked, instead choose any number of
//! target creatures you own. Return each chosen creature to your hand. At the beginning of the next upkeep, create
//! a 4/4 white Bird Soldier creature token with flying for each creature returned to your hand this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheEaglesAreComing.TheEaglesAreComingEffect", apply).effects(&[
    "new ReturnToHandTargetEffect()",
    "new CreateTokenEffect(new BirdSoldier44Token())",
])];

/// `ReturnToHandTargetEffect`, then a delayed trigger at the next upkeep creating a Bird Soldier for each target
/// (the target pointer's targets still read their last-known battlefield objects once returned)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.objs(0).len();
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let mut create = cx.effects.get(1)?.clone();
    if let EffectDef::CreateTokenEffect { amount, .. } = &mut create {
        *amount = crate::card::Amount::Fixed(n as i32);
    }
    let tokens = if n == 1 { "token" } else { "tokens" };
    let rule = format!(
        "Create {} 4/4 white Bird Soldier creature {tokens} with flying at the beginning of the next turn's upkeep.",
        log::number(n)
    );
    cx.run(game, d, &[EffectDef::CreateNextUpkeepDelayedEffect { effects: vec![create], rule }]);
    Some(())
}

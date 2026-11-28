//! Flourishing Grapple: "Target creature or planeswalker an opponent controls that's red or white loses all abilities
//! until end of turn. Target creature you control deals damage equal to its power to that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FlourishingGrapple.FlourishingGrappleEffect", apply)];

/// the second target (tag 2) deals damage equal to its power to the first (tag 1), both still there
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    let creature = cx.obj(1)?;
    let power = game.power_toughness(creature).0;
    deal_damage(game, creature, Target::Obj(target), power, false);
    Some(())
}

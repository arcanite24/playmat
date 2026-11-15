//! Spoils of the Hunt: "Target creature you control gets +1/+0 until end of turn for each mana from a Treasure that
//! was spent to cast this spell. Then that creature deals damage equal to its power to target creature an opponent
//! controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpoilsOfTheHunt.SpoilsOfTheHuntEffect", apply)];

/// +X/+0 until end of turn on the first target, X the Treasure mana spent to cast the spell
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = game.obj(cx.source_id()).treasure_spent as i32;
    let Some(o) = cx.obj(0) else { return Some(()) };
    let e = EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(n),
        toughness: crate::card::Amount::Fixed(0),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    (!d.stopped()).then_some(())
}

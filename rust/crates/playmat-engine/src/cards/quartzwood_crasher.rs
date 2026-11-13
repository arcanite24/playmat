//! Quartzwood Crasher: "Whenever one or more creatures you control with trample deal combat damage to a player,
//! create an X/X green Dinosaur Beast creature token with trample, where X is the amount of damage those creatures
//! dealt to that player."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("QuartzwoodCrasher.QuartzwoodCrasherEffect", apply).tokens(&["DinosaurBeastToken"])];

/// `new DinosaurBeastToken(SavedDamageValue.MUCH)`: an X/X token, X the batch's damage (the trigger's `x`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: crate::card::Amount::Fixed(cx.item.x) };
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    Some(())
}

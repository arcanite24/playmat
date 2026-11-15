//! Multiform Wonder: "When Multiform Wonder enters, you get {E}{E}{E}. Pay {E}: Multiform Wonder gains your choice of
//! flying, vigilance, or lifelink until end of turn. Pay {E}: Multiform Wonder gets +2/-2 or -2/+2 until end of
//! turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MultiformWonder.MultiformWonder2Effect", apply).effects(&[
    "new BoostSourceEffect(2, -2, Duration.EndOfTurn)",
    "new BoostSourceEffect(-2, 2, Duration.EndOfTurn)",
])];

/// "Should <it> get -2/+2 instead of +2/-2?", then that boost until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let message = format!("Should {} get -2/+2 instead of +2/-2?", log::object(game, cx.source_id()));
    let swap = choose_use(game, d, cx.controller(), &message)?;
    cx.run(game, d, &cx.effects[usize::from(swap)..usize::from(swap) + 1]);
    (!d.stopped()).then_some(())
}

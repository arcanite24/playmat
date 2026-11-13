//! Aunt May: "Whenever another creature you control enters, you gain 1 life. If it's a Spider, put a +1/+1 counter
//! on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AuntMay.AuntMayEffect", apply)];

/// The creature that entered (the target pointer), if it's still a Spider on the battlefield, gets a +1/+1 counter
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || !game.has_subtype(o, "Spider") {
        return Some(());
    }
    let counter = crate::card::CounterDef { name: "+1/+1".into(), amount: 1 };
    cx.run(game, d, &[EffectDef::AddCountersTargetEffect { counter, value: None, second: false }]);
    Some(())
}

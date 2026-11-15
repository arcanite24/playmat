//! Repeat Offender: "{2}{B}: If this creature is suspected, put a +1/+1 counter on it. Otherwise, suspect it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RepeatOffender.RepeatOffenderEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    if game.obj(cx.source_id()).suspected() {
        let counter = crate::card::CounterDef { name: "+1/+1".into(), amount: 1 };
        cx.run(game, d, &[EffectDef::AddCountersSourceEffect { counter, amount: None }]);
    } else {
        game.obj_mut(cx.source_id()).set_suspected(true);
    }
    Some(())
}

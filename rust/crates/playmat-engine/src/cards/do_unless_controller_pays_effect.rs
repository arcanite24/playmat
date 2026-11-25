//! `DoUnlessControllerPaysEffect(effect, cost)`: "… unless you pay the cost" (Magmatic Sprinter: "return this to its
//! owner's hand unless you remove two oil counters from it"). Only the forms written here translate.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DoUnlessControllerPaysEffect", apply)
    .args(&[
        "one_of:new ReturnToHandSourceEffect(true)",
        "one_of:new RemoveCountersSourceCost(CounterType.OIL.createInstance(2))",
    ])
    .effects(&["new ReturnToHandSourceEffect(true)"])];

/// "Pay <cost> to prevent (<effect's text>)?" when the cost can be paid (the source on the battlefield with two oil
/// counters); paid, nothing else happens, else the effect
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let name = game.obj(src).name().to_string();
    // (`game.getPermanent(source.getSourceId())`: whatever object it is now)
    if game.obj(src).zone == Zone::Battlefield &&game.obj(src).counter("oil") >= 2 {
        let message = format!("Pay remove two oil counters from {name} to prevent (return {name} to its owner's hand)?");
        if choose_use(game, d, you, &message)? {
            game.obj_mut(src).add_counters("oil", -2);
            return Some(());
        }
    }
    cx.run(game, d, cx.effects);
    (!d.stopped()).then_some(())
}

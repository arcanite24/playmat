//! Strixhaven Stadium: "{T}: Add {C}. Put a point counter on Strixhaven Stadium. Whenever a creature deals combat
//! damage to you, remove a point counter from Strixhaven Stadium. Whenever a creature you control deals combat damage
//! to an opponent, put a point counter on Strixhaven Stadium. Then if it has ten or more point counters on it, remove
//! them all and that player loses the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StrixhavenStadium.StrixhavenStadiumEffect", apply)
    .effects(&["new RemoveAllCountersSourceEffect(CounterType.POINT)", "new LoseGameTargetPlayerEffect()"])];

/// The source if it's still on the battlefield, else as it last was there: ten or more point counters, all removed
/// (only from the permanent still there), then the player the trigger points at loses
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.obj(src).counter("point") < 10 {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        cx.run(game, d, &cx.effects[..1]);
        if d.stopped() {
            return None;
        }
    }
    cx.run(game, d, &cx.effects[1..]);
    if d.stopped() {
        return None;
    }
    Some(())
}

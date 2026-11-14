//! Tragic Lesson: "Draw two cards. Then discard a card unless you return a land you control to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TragicLesson.TragicLessonEffect", apply)];

/// asked first (whether or not a land can be returned); a returned land saves the discard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cost: CostDef = serde_json::from_value(serde_json::json!({
        "class": "ReturnToHandChosenControlledPermanentCost",
        "amount": 1,
        "filter": [{"p": "type", "t": "LAND"}, {"p": "controller", "who": "you"}]
    }))
    .ok()?;
    if choose_use(game, d, you, "Return a land you control to its owner's hand?")?
        && can_pay_cost(game, you, cx.source_id(), &cost)
        && pay_cost(game, d, you, cx.source_id(), &cost)
    {
        return Some(());
    }
    if d.stopped() {
        return None;
    }
    discard(game, d, you, 1);
    Some(())
}

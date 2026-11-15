//! Wildborn Preserver: "Flash. Reach. Whenever another non-Human creature you control enters, you may pay {X}. When
//! you do, put X +1/+1 counters on Wildborn Preserver."

use super::*;
use crate::card::{CounterDef, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::effect("WildbornPreserver.WildbornPreserverCreateReflexiveTriggerEffect", apply)];

/// "Pay {X}?", `announceX(0, MAX)` and the cost paid; paid, `fireReflexiveTriggeredAbility` of
/// `AddCountersSourceEffect(P1P1 X)` ("Put X +1/+1 counters on {this}.")
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Pay {X}?")? {
        return Some(());
    }
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let cost = ManaCost { generic: x as u32, ..ManaCost::default() };
    let before = game.clone();
    if !pay_mana(game, d, you, &cost) {
        if d.stopped() {
            return None;
        }
        *game = before;
        return Some(());
    }
    let counter = CounterDef { name: "+1/+1".into(), amount: x };
    let delayed = Delayed {
        rule: "Put X +1/+1 counters on {this}.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::AddCountersSourceEffect { counter, amount: None }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), x, ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

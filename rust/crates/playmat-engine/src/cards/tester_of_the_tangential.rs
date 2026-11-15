//! Tester of the Tangential: "Increment. At the beginning of combat on your turn, you may pay {X}. When you do, move
//! X +1/+1 counters from this creature onto another target creature."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TesterOfTheTangential.TesterOfTheTangentialEffect", apply)
        .filters(&["StaticFilters.FILTER_ANOTHER_TARGET_CREATURE"]),
    // the reflexive trigger's `MoveCountersFromSourceToTargetEffect(CounterType.P1P1, X)`, X its item's
    Handler::effect("TesterOfTheTangential.MoveCountersFromSourceToTargetEffect", move_counters),
];

/// "Pay {X}?", `announceX(0, MAX)` and the cost paid; paid, `fireReflexiveTriggeredAbility` with X
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
    let target = TargetDef::TargetPermanent {
        filter: cx.filters.first()?.clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CardEffect", "handler": "TesterOfTheTangential.MoveCountersFromSourceToTargetEffect"
    }))
    .ok()?;
    let delayed = Delayed {
        rule: format!("Move {x} +1/+1 counters from {{this}} onto another target creature."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), x, ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// X +1/+1 counters onto the target, then off this, when this is still there with at least X of them
fn move_counters(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.item.x;
    let src = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) || to == src || game.obj(src).counter("+1/+1") < n || n < 1 {
        return Some(());
    }
    add_counters_by(game, to, "+1/+1", n, Some(cx.controller()));
    let o = game.obj_mut(src);
    o.add_counters("+1/+1", -n);
    o.counters.retain(|(_, k)| *k > 0);
    Some(())
}

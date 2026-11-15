//! Systems Override: "Gain control of target artifact or creature until end of turn. Untap that permanent. It gains
//! haste until end of turn. If it's a Spacecraft, put ten charge counters on it. If you do, remove ten charge counters
//! from it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SystemsOverride.SystemsOverrideEffect", apply),
    Handler::effect("SystemsOverride.SystemsOverrideRemoveEffect", remove),
];

/// A Spacecraft still targeted gets ten charge counters; if any were put, a delayed trigger at the next end step
/// removes ten from it while it's still that permanent (`FixedTarget`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || !game.has_subtype(o, "Spacecraft") {
        return Some(());
    }
    let before = game.obj(o).counter("charge");
    put_counters(game, o, "charge", 10, you);
    if game.obj(o).counter("charge") <= before {
        return Some(());
    }
    let handler = serde_json::from_value(serde_json::json!("SystemsOverride.SystemsOverrideRemoveEffect")).ok()?;
    let e = EffectDef::CardEffect { handler, args: Vec::new(), tokens: Vec::new(), effects: Vec::new(), filters: Vec::new() };
    let delayed = Delayed {
        rule: "At the beginning of the next end step, remove ten charge counters from it.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: vec![(o, game.obj(o).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// `RemoveCounterTargetEffect(CounterType.CHARGE.createInstance(10))`: up to ten, as many as it has
fn remove(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let n = game.obj(o).counter("charge").min(10);
        if n > 0 {
            let ob = game.obj_mut(o);
            ob.add_counters("charge", -n);
            ob.counters.retain(|(_, k)| *k > 0);
        }
    }
    Some(())
}

//! Plasm Capture: "Counter target spell. At the beginning of your next first main phase, add X mana in any
//! combination of colors, where X is that spell's mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("PlasmCapture.PlasmCaptureCounterEffect", apply),
    Handler::effect("PlasmCapture.PlasmCaptureDelayedManaEffect", add_mana),
];

/// the spell's mana value (its X included) as it is on the stack, then the counter; the delayed trigger is made
/// whether or not it was countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let Some(x) = game.stack.iter().rev().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.x) else {
        return Some(());
    };
    let mv = {
        let c = game.obj(spell).def.printed();
        c.mana_value() as i32 + x * c.x as i32
    };
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if d.stopped() {
        return None;
    }
    let handler = serde_json::from_value(serde_json::json!("PlasmCapture.PlasmCaptureDelayedManaEffect")).ok()?;
    let add = EffectDef::CardEffect {
        handler,
        args: vec![mv.to_string()],
        tokens: Vec::new(),
        effects: Vec::new(),
        filters: Vec::new(),
    };
    let rule = format!("At the beginning of your next first main phase, add {mv} mana in any combination of colors.");
    crate::rules::next_main_delayed(game, cx.source, cx.controller(), vec![add], rule, "precombat");
    Some(())
}

/// `AddManaInAnyCombinationEffect(mana)`: `getMultiAmount` over W, U, B, R, G (all of it on the first by default)
fn add_mana(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_int(0).unwrap_or(0).max(0);
    let you = cx.controller();
    let mut defaults = vec![0; 5];
    defaults[0] = n;
    let values = multi_amount(game, d, you, defaults, vec![(0, n); 5], (n, n), (None, Vec::new()))?;
    let mut colored = [0u32; 5];
    for (k, v) in values.into_iter().enumerate().take(5) {
        colored[k] = v.max(0) as u32;
    }
    let e = EffectDef::BasicManaEffect { colored, colorless: 0, until_end_of_turn: false };
    cx.run_on(game, d, &[e], Vec::new());
    Some(())
}

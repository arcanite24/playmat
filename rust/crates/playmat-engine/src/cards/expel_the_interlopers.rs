//! Expel the Interlopers: "Choose a number between 0 and 10. Destroy all creatures with power greater than or equal
//! to the chosen number."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExpelTheInterlopers.ExpelTheInterlopersEffect", apply)];

/// `Player.getAmount(0, 10)`, then `DestroyAllEffect` of the creatures with at least that power
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: 10 }) {
            Answer::Int(n) if (0..=10).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "DestroyAllEffect",
        "filter": [{"p": "type", "t": "CREATURE"}, {"cmp": "OR_GREATER", "n": n, "p": "power"}]
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}

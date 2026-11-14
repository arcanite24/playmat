//! Kindred Judgment: "Choose a creature type. Destroy all creatures that aren't of the chosen type."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KindredJudgment.KindredJudgmentEffect", apply)];

/// `ChoiceCreatureType`, then `DestroyAllEffect` of the creatures not of that type
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let q = Query::Choice { message: "Choose a creature type".into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "DestroyAllEffect",
        "filter": [{"p": "type", "t": "CREATURE"}, {"p": "not", "x": {"p": "subtype", "s": chosen}}]
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}

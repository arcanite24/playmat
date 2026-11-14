//! Witch's Vengeance: "Creatures of the creature type of your choice get -3/-3 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WitchsVengeance.WitchsVengeanceEffect", apply)];

/// `ChoiceCreatureType`, then `BoostAllEffect(-3, -3, EndOfTurn)` over creatures of that type
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
        "class": "BoostAllEffect", "power": -3, "toughness": -3, "duration": "EndOfTurn", "exclude_source": false,
        "filter": [{"p": "type", "t": "CREATURE"}, {"p": "subtype", "s": chosen}]
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}

//! Invoke the Ancients: "Create two 4/5 green Spirit creature tokens. For each of them, put your choice of a
//! vigilance counter, a reach counter, or a trample counter on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("InvokeTheAncients.InvokeTheAncientsEffect", apply).tokens(&["SpiritGreenToken"])];

/// the two Spirits, then for each (in the order made) the required `ChoiceImpl` of vigilance, reach or trample
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before = game.battlefield.clone();
    cx.create_tokens(game, d, &cx.tokens[0], 2);
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    let choices: Vec<String> = ["Vigilance", "Reach", "Trample"].iter().map(|s| s.to_string()).collect();
    for t in made {
        if game.obj(t).zone != Zone::Battlefield {
            continue;
        }
        let q = Query::Choice { message: "Choose vigilance, reach, or trample counter".into(), choices: choices.clone() };
        let chosen = loop {
            match ask(game, d, you, &q) {
                Answer::Text(c) if choices.contains(&c) => break c,
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        };
        put_counters(game, t, &chosen.to_lowercase(), 1, you);
    }
    Some(())
}

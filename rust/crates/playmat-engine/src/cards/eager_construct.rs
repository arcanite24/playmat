//! Eager Construct: "When Eager Construct enters the battlefield, each player may scry 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EagerConstruct.EagerConstructEffect", apply)];

/// Each player in range from you, in turn order: "Scry 1? ..." and the scry on a yes
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let q = Query::YesNo {
        message: "Scry 1? <i>(Look at the top card of your library. You may put that card on the bottom of your library.)</i>"
            .into(),
    };
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let yes = loop {
            match ask(game, d, p, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return None,
                _ => {}
            }
        };
        if yes && !scry(game, d, p, 1, &rule) {
            return None;
        }
    }
    Some(())
}

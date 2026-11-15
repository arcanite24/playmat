//! Myr Custodian: "When Myr Custodian enters the battlefield, scry 2. Then each opponent may scry 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MyrCustodian.MyrCustodianScryEffect", apply)];

/// Each opponent in turn order: "Scry 1?" and the scry on a yes
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let q = Query::YesNo { message: "Scry 1?".into() };
    for i in 1..game.seats() {
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

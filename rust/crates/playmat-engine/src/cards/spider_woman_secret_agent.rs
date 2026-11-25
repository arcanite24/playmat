//! Spider-Woman, Secret Agent: "When Spider-Woman enters, tap target creature an opponent controls. That creature
//! can't become untapped for as long as you control Spider-Woman."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SpiderWomanSecretAgent.SpiderWomanSecretAgentEffect", apply)];

/// `UNTAP` of the target replaced while you control the source (`Duration.WhileControlled`): the engine's
/// `dont_untap_while` under its controller, read by the untap step. (Java's replacement also stops an effect that
/// untaps it.) With the source gone already, it lasts no time at all.
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(cx.source_id()).controller != cx.controller() {
        return Some(());
    }
    let zcc = game.obj(t).zcc;
    let (src, szcc) = cx.source;
    game.dont_untap_while.push((t, zcc, src, szcc, Some(cx.controller())));
    Some(())
}

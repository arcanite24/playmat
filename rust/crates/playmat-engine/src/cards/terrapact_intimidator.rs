//! Terrapact Intimidator: "When this creature enters, target opponent may have you create two Lander tokens. If they
//! don't, put two +1/+1 counters on this creature."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TerrapactIntimidator.TerrapactIntimidatorEffect", apply).tokens(&["LanderToken"])];

/// the opponent is asked; yes: two Landers for you, else two counters on it while it's still there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let opponent = cx.player(0)?;
    let you = cx.controller();
    let q = format!("Have {} create two Lander tokens?", log::player(you));
    if choose_use(game, d, opponent, &q)? {
        cx.create_tokens(game, d, &cx.tokens[0], 2);
        return (!d.stopped()).then_some(());
    }
    if cx.source_on_battlefield(game) {
        add_counters_by(game, cx.source_id(), "+1/+1", 2, Some(you));
    }
    Some(())
}

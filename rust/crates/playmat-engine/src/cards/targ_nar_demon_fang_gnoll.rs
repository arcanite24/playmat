//! Targ Nar, Demon-Fang Gnoll: "{2}{R}{G}: Double Targ Nar's power and toughness until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TargNarDemonFangGnoll.TargNarDemonFangGnollEffect", apply)];

/// `BoostSourceEffect` of its power and toughness as the ability resolves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let (p, t) = game.power_toughness(cx.source_id());
    boost(game, cx.source_id(), p, t, Duration::EndOfTurn);
    Some(())
}

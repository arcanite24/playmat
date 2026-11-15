//! Riverwheel Sweep: "Tap target creature. Put three stun counters on it. Exile the top two cards of your library.
//! Choose one of them. Until the end of your next turn, you may play that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiverwheelSweep.RiverwheelSweepEffect", apply)];

/// The engine's exile-the-top-cards-and-choose-one, playable until the end of your next turn (one card exiled is
/// that card: `getRandom` of one draws no random number)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::ExileTopXMayPlayUntilEffect {
        amount: Amount::Fixed(2),
        choose_one: true,
        duration: "UntilEndOfYourNextTurn".into(),
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    Some(())
}

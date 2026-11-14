//! Unyielding Gatekeeper: "When Unyielding Gatekeeper is turned face up, exile another target nonland permanent. If
//! you controlled it, return it to the battlefield tapped. Otherwise, its controller creates a 2/2 white and blue
//! Detective creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnyieldingGatekeeper.UnyieldingGatekeeperEffect", apply).effects(&[
    "new ExileThenReturnTargetEffect(false, false, PutCards.BATTLEFIELD_TAPPED)",
    "new ExileTargetEffect()",
    "new CreateTokenControllerTargetEffect(new DetectiveToken())",
])];

/// Yours: exiled and back tapped; else exiled, and its controller (as it last was) creates the Detective
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    if game.obj(o).controller == cx.controller() {
        cx.run(game, d, &cx.effects[..1]);
    } else {
        cx.run(game, d, &cx.effects[1..]);
    }
    Some(())
}

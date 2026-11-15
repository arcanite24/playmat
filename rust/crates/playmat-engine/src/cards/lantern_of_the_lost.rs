//! Lantern of the Lost: "When Lantern of the Lost enters the battlefield, exile target card from a graveyard. {1},
//! {T}, Exile Lantern of the Lost: Exile all cards from all graveyards, then draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LanternOfTheLost.LanternOfTheLostEffect", apply)
    .effects(&["new ExileGraveyardAllPlayersEffect()", "new DrawCardSourceControllerEffect(1)"])];

/// every graveyard's cards exiled together (`moveCards`), then you draw a card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run_on(game, d, &cx.effects, Vec::new());
    (!d.stopped()).then_some(())
}

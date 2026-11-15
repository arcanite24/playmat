//! Flunk: "Target creature gets -X/-X until end of turn, where X is 7 minus the number of cards in that creature's
//! controller's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Flunk.FlunkEffect", apply)];

/// X is counted as it resolves (at least 0), then the boost lasts until end of turn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(t).controller;
    let x = (7 - game.player(p).hand.len() as i32).max(0);
    boost(game, t, -x, -x, crate::card::Duration::EndOfTurn);
    Some(())
}

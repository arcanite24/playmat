//! Triskaidekaphobia: "At the beginning of your upkeep, choose one — Each player with exactly 13 life loses the game,
//! then each player gains 1 life; or each player with exactly 13 life loses the game, then each player loses 1 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("Triskaidekaphobia.TriskaidekaphobiaGainLifeEffect", apply)];

/// each player in range with exactly 13 life loses (`Player.lost`), in turn order from you
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if !game.player(p).lost && game.player(p).life == 13 {
            cx.run_as(game, d, &[EffectDef::LoseGameSourceControllerEffect], p);
        }
    }
    (!d.stopped()).then_some(())
}

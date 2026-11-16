//! Triskaidekaphobia: "At the beginning of your upkeep, choose one — Each player with exactly 13 life loses the game,
//! then each player gains 1 life; or each player with exactly 13 life loses the game, then each player loses 1 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("Triskaidekaphobia.TriskaidekaphobiaGainLifeEffect", apply)];

/// each player in range with exactly 13 life loses (`Player.lost`), in turn order from you. With one player left the
/// game ends there, as it resolves (as `LoseGameTargetPlayerEffect`): the life gained or lost after never happens
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost || game.player(p).won || game.player(p).life != 13 {
            continue;
        }
        game.player_mut(p).lost = true;
        if log::kept(game) {
            let line = format!("{} has lost the game", log::player(p));
            log::inform(game, line);
        }
        let alive: Vec<Seat> = (0..game.seats()).filter(|s| !game.player(*s).lost).collect();
        if let ([w], false) = (&alive[..], game.over) {
            let w = *w;
            if log::kept(game) {
                let line = format!("{} has won the game", log::player(w));
                log::inform(game, line);
            }
            game.player_mut(w).won = true;
            game.over = true;
            return None;
        }
    }
    (!d.stopped()).then_some(())
}

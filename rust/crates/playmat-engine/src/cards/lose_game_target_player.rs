//! `LoseGameTargetPlayerEffect` (Vraska, Golgari Queen's emblem: "Whenever a creature you control deals combat
//! damage to a player, that player loses the game.")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LoseGameTargetPlayerEffect", apply)];

/// `PlayerImpl.lostForced`: the player pointed at, still in and not the winner, loses; with one player left the
/// game ends there, as it resolves (its end traced with the ability still on the stack, then that one has won)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    if game.player(p).lost || game.player(p).won {
        return Some(());
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
    Some(())
}

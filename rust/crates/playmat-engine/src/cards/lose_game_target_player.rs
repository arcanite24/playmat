//! `LoseGameTargetPlayerEffect` (Vraska, Golgari Queen's emblem: "Whenever a creature you control deals combat
//! damage to a player, that player loses the game.")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LoseGameTargetPlayerEffect", apply)];

/// `PlayerImpl.lostForced`: the player pointed at, still in and not the winner, loses (the state-based check then
/// ends the game with one player left)
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
    Some(())
}

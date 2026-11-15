//! Step Between Worlds: "Each player may shuffle their hand and graveyard into their library. Each player who does
//! draws seven cards. Exile Step Between Worlds."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StepBetweenWorlds.StepBetweenWorldsEffect", apply)];

/// each player in range from you is asked; then each who said yes, in that order, shuffles their hand and graveyard
/// into their library (`shuffleCardsToLibrary`: nothing with both empty) and draws seven
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut players = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        if choose_use(game, d, p, "Shuffle your hand and graveyard into your library and draw seven cards?")? {
            players.push(p);
        }
    }
    for p in players {
        let player = game.player(p);
        let cards: Vec<ObjId> = player.hand.iter().chain(player.graveyard.iter()).copied().collect();
        if !cards.is_empty() {
            for c in cards {
                to_library(game, c, true);
            }
            let mut library = game.player(p).library.clone();
            game.rng.shuffle(&mut library);
            game.player_mut(p).library = library;
        }
        draw(game, p, 7);
    }
    Some(())
}

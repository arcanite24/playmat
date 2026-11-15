//! Turtles in Time: "Return all creatures to their owners' hands. Each player may shuffle their hand and graveyard
//! into their library, then each player who does draws seven cards. Exile Turtles in Time."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TurtlesInTime.TurtlesInTimeEffect", apply)];

/// each player in range from you is asked, then each who said yes shuffles (`shuffleCardsToLibrary`) and draws
/// seven, in that order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut players = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let yes = choose_use(game, d, p, "Shuffle your hand and graveyard into your library and draw seven cards?")?;
        if log::kept(game) {
            let line = format!("{} chooses {}to shuffle and draw", log::player(p), if yes { "" } else { "not " });
            log::inform(game, line);
        }
        if yes {
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

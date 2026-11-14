//! Doomsday Excruciator: "When this creature enters, if it was cast, each player exiles all but the bottom six cards
//! of their library face down."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DoomsdayExcruciator.DoomsdayExcruciatorEffect", apply)];

/// Every player's library but its bottom six, exiled together (one `moveCards`), then turned face down
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let library = &game.player(p).library;
        let n = library.len().saturating_sub(6);
        cards.extend(library.iter().take(n).copied());
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).face_down = true;
        }
    }
    Some(())
}

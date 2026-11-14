//! Doomsday Excruciator: "When Doomsday Excruciator enters, if it was cast, each player exiles all but the bottom
//! six cards of their library face down."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DoomsdayExcruciator.DoomsdayExcruciatorEffect", apply)];

/// every player in range, starting with you: their library's top cards but six, all exiled face down by you
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    let n = game.seats();
    for p in (0..n).map(|k| (you + k) % n).filter(|p| !game.player(*p).lost) {
        let lib = &game.player(p).library;
        let all_but_six = lib.len().saturating_sub(6);
        cards.extend(lib.iter().take(all_but_six).copied());
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

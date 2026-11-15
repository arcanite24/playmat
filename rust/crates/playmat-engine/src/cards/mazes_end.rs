//! Maze's End: "{3}, {T}, Return Maze's End to its owner's hand: Search your library for a Gate card, put it onto the
//! battlefield, then shuffle. If you control ten or more Gates with different names, you win the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MazesEnd.MazesEndEffect", apply)];

/// `GatesWithDifferentNamesYouControlCount`: ten or more names among the Gates you control, and you win
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut names: Vec<String> = Vec::new();
    for o in game.battlefield.iter().copied() {
        if game.obj(o).controller == you && game.has_subtype(o, "Gate") {
            let name = game.obj(o).name().to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    if names.len() >= 10 {
        win_game(game, you);
    }
    Some(())
}

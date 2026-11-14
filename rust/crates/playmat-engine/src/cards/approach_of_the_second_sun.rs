//! Approach of the Second Sun: "If this spell was cast from your hand and you've cast another spell named Approach
//! of the Second Sun this game, you win the game. Otherwise, put Approach of the Second Sun into its owner's library
//! seventh from the top and you gain 7 life."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ApproachOfTheSecondSun.ApproachOfTheSecondSunEffect", apply)];

const NAME: &str = "Approach of the Second Sun";

/// The spell still on the stack (Java: `getStack().getSpell(sourceId)`, else nothing happens). Cast from hand with
/// two or more Approaches cast this game (`ApproachOfTheSecondSunWatcher` counts this one too): you win. Otherwise
/// 7 life, then `putCardOnTopXOfLibrary(card, 7, withName)`: on the bottom when the library has fewer than six
/// cards, else seventh from the top, with its log line
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let id = cx.source_id();
    if game.obj(id).zone != Zone::Stack {
        return Some(());
    }
    let cast = game.player(you).cast_this_game.iter().filter(|n| *n == NAME).count();
    if game.obj(id).cast_from == Some(Zone::Hand) && cast > 1 {
        win_game(game, you);
        return Some(());
    }
    gain_life(game, you, 7);
    let owner = game.obj(id).owner;
    if game.player(owner).library.len() + 1 < 7 {
        to_library(game, id, false);
        return Some(());
    }
    game.move_to(id, Zone::Library);
    if game.obj(id).zone != Zone::Library {
        return Some(());
    }
    let lib = &mut game.player_mut(owner).library;
    lib.retain(|x| *x != id);
    lib.insert(6, id);
    if log::kept(game) {
        let line =
            format!("{} is put into {}'s library seventh from the top", log::object(game, id), log::player(owner));
        log::inform(game, line);
    }
    Some(())
}

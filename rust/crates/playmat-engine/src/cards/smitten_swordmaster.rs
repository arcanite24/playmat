//! Smitten Swordmaster // Curry Favor: "You gain X life and each opponent loses X life, where X is the number of
//! Knights you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SmittenSwordmaster.CurryFavorEffect", apply)];

/// the Knights counted once, then the gain, then each opponent's loss in turn order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Knight")).count()
        as i32;
    gain_life(game, you, x);
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if !game.player(p).lost {
            lose_life(game, p, x);
        }
    }
    Some(())
}

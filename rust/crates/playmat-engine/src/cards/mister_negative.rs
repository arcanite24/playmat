//! Mister Negative: "Darkforce Inversion — When Mister Negative enters, you may exchange life totals with target
//! opponent. If you lose life this way, draw that many cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MisterNegative.MisterNegativeEffect", apply)];

/// `exchangeLife`: each `setLife` a gain or a loss, yours first; then the draw for what you lost
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let (mine, theirs) = (game.player(you).life, game.player(p).life);
    if mine != theirs {
        set_life(game, you, theirs);
        set_life(game, p, mine);
    }
    let lost = mine - game.player(you).life;
    if lost > 0 {
        draw(game, you, lost as usize);
    }
    Some(())
}

/// `PlayerImpl.setLife`: a gain or a loss of the difference
fn set_life(game: &mut Game, p: Seat, to: i32) {
    let life = game.player(p).life;
    if to > life {
        gain_life(game, p, to - life);
    } else if to < life {
        lose_life(game, p, life - to);
    }
}

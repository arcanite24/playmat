//! `SetPlayerLifeSourceEffect(n)`: "your life total becomes N" (Form of the Dinosaur, Angel of Grace, Captive
//! Audience): `PlayerImpl.setLife` for the controller, a gain or a loss of the difference (rule 118.5).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SetPlayerLifeSourceEffect", apply).args(&["int"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    set_life(game, cx.controller(), cx.arg_int(0)?);
    Some(())
}

/// `PlayerImpl.setLife`: a gain or a loss of the difference
pub(super) fn set_life(game: &mut Game, p: Seat, to: i32) {
    let life = game.player(p).life;
    if to > life {
        gain_life(game, p, to - life);
    } else if to < life {
        lose_life(game, p, life - to);
    }
}

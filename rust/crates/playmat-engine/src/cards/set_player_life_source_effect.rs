//! `SetPlayerLifeSourceEffect(n)`: "your life total becomes N" (Form of the Dinosaur, Angel of Grace, Captive
//! Audience): `PlayerImpl.setLife` for the controller, a gain or a loss of the difference (rule 118.5). Also with a
//! `DynamicValue` (Revival // Revenge's "double your life total", `MultipliedValue(ControllerLifeCount.instance, 2)`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SetPlayerLifeSourceEffect", apply).args(&["int_or_amount"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let to = cx.arg_int(0).or_else(|| cx.arg_amount(game, 0))?;
    set_life(game, cx.controller(), to);
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

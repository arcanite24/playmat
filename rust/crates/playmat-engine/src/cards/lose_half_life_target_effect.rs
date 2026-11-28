//! `LoseHalfLifeTargetEffect`: "that player loses half their life, rounded up" (Havoc Festival, Rush of Dread,
//! Unstoppable Slasher)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LoseHalfLifeTargetEffect", apply)];

/// the target player (an upkeep trigger's: the active player) loses `ceil(life / 2)`, if that's more than 0
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let life = game.player(p).life;
    let n = (life as f32 / 2.0).ceil() as i32;
    if n > 0 {
        lose_life(game, p, n);
    }
    Some(())
}

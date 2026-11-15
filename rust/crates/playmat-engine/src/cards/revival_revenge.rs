//! Revival // Revenge: "Return target creature card with mana value 3 or less from your graveyard to the
//! battlefield. // Double your life total. Target opponent loses half their life, rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RevivalRevenge.RevivalRevengeEffect", apply)];

/// You gain life equal to your life total, then the target opponent loses half their life, rounded up
/// (`Math.floorDiv(life, 2) + life % 2`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let life = game.player(you).life;
    gain_life(game, you, life);
    let theirs = game.player(p).life;
    lose_life_by(game, p, theirs.div_euclid(2) + theirs % 2, Some(cx.source_id()), false);
    Some(())
}

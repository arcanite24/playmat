//! Tempest Caller: "When Tempest Caller enters the battlefield, tap all creatures target opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TempestCaller.TapAllTargetPlayerControlsEffect", apply)];

/// `TapAllTargetPlayerControlsEffect(FILTER_PERMANENT_CREATURES)`: each creature the target player controls is
/// tapped, in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        tap(game, o);
    }
    Some(())
}

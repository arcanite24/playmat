//! Kiora Bests the Sea God, chapter II: "Tap all nonland permanents target opponent controls. They don't untap
//! during their controller's next untap step."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::effect("KioraBestsTheSeaGod.KioraBestsTheSeaGodEffect", apply)];

/// `DontUntapInControllersNextUntapStepTargetEffect` on every nonland permanent the target player controls now
/// (`getAllActivePermanents(FILTER_PERMANENTS_NON_LAND, firstTarget, game)`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let all: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land))
        .collect();
    for o in all {
        let zcc = game.obj(o).zcc;
        if !game.skip_next_untap.contains(&(o, zcc)) {
            game.skip_next_untap.push((o, zcc));
        }
    }
    Some(())
}

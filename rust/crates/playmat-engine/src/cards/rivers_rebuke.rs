//! River's Rebuke: "Return all nonland permanents target player controls to their owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RiversRebuke.RiversRebukeReturnToHandEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && !game.has_type(*o, CardType::Land))
        .collect();
    for o in theirs {
        game.move_to(o, Zone::Hand);
    }
    Some(())
}

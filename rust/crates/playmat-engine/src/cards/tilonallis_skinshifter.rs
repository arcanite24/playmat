//! Tilonalli's Skinshifter: "Whenever Tilonalli's Skinshifter attacks, it becomes a copy of another target
//! nonlegendary attacking creature until end of turn."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("TilonallisSkinshifter.TilonallisSkinshifterCopyEffect", apply)];

/// `game.copyPermanent(Duration.EndOfTurn, target, source, new EmptyCopyApplier())`
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(src).zone != Zone::Battlefield || game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let copy = game.obj(of).copiable();
    become_copy_until_end_of_turn(game, src, copy);
    Some(())
}

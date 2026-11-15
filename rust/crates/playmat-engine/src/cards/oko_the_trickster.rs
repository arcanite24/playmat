//! Oko, the Trickster: "0: Until end of turn, Oko, the Trickster becomes a copy of target creature you control.
//! Prevent all damage that would be dealt to him this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OkoTheTrickster.OkoTheTricksterCopyEffect", apply)];

/// `game.copyPermanent(Duration.EndOfTurn, target, source, new EmptyCopyApplier())`: both still on the battlefield
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

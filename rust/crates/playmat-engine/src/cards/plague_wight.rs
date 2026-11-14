//! Plague Wight: "Whenever Plague Wight becomes blocked, each creature blocking it gets -1/-1 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PlagueWight.PlagueWightEffect", apply)];

/// The creatures blocking it as this resolves (`BlockingAttackerIdPredicate`), while it's still there
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let me = cx.source_id();
    if game.obj(me).zone != Zone::Battlefield {
        return None;
    }
    let blockers: Vec<ObjId> = game.obj(me).blocked_by.clone();
    for b in blockers {
        if game.obj(b).zone == Zone::Battlefield {
            boost(game, b, -1, -1, crate::card::Duration::EndOfTurn);
        }
    }
    Some(())
}

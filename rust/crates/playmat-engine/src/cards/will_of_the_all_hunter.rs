//! Will of the All-Hunter: "Target creature gets +2/+2 until end of turn. If it's blocking, instead put two +1/+1
//! counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WillOfTheAllHunter.WillOfTheAllHunterEffect", apply)];

/// Java's `getBlocking() > 0`: declared as a blocker this combat
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    if game.obj(o).blocks > 0 {
        add_counters_by(game, o, "+1/+1", 2, Some(cx.controller()));
    } else {
        boost(game, o, 2, 2, crate::card::Duration::EndOfTurn);
    }
    Some(())
}

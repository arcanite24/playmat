//! Bulk Up: "Double target creature's power until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BulkUp.BulkUpEffect", apply)];

/// +X/+0 until end of turn, X its power now; nothing with power 0
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.power_toughness(o).0;
    if p != 0 {
        boost(game, o, p, 0, Duration::EndOfTurn);
    }
    Some(())
}

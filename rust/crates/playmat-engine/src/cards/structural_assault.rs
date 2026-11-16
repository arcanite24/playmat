//! Structural Assault: "Destroy all artifacts, then Structural Assault deals damage to each creature equal to the
//! number of artifacts that were put into graveyards from the battlefield this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("StructuralAssault.StructuralAssaultEffect", apply),
    Handler::value("StructuralAssault.StructuralAssaultValue", artifacts_died),
];

/// `StructuralAssaultWatcher`: the artifacts that died this turn (tokens too)
fn artifacts_died(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.dies.iter().filter(|(_, artifact, _, _)| *artifact).count() as i32
}

/// that much damage to each creature on the battlefield, in battlefield order; nothing when none died
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = artifacts_died(game, cx.controller(), cx.source_id(), 0);
    if n <= 0 {
        return Some(());
    }
    let creatures: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|p| game.has_type(*p, CardType::Creature)).collect();
    for p in creatures {
        if game.obj(p).zone == Zone::Battlefield {
            deal_damage(game, cx.source_id(), Target::Obj(p), n, false);
        }
    }
    Some(())
}

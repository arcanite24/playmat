//! Paradoxical Outcome: "Return any number of target nonland, nontoken permanents you control to their owners'
//! hands. Draw a card for each card returned to your hand this way."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ParadoxicalOutcome.ParadoxicalOutcomeEffect", apply),
    Handler::value("ParadoxicalOutcome.ParadoxicalOutcomeNumber", returned).args(&["bool"]),
];

/// The targets still legal, to their owners' hands together
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Hand);
        }
    }
    Some(())
}

/// the number of targets the effect returned (Java stores the legal targets' count as it returns them): the
/// resolving spell's targets now in a hand, one zone change on
fn returned(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some(item) = game.resolving_item.as_ref() else { return 0 };
    let (Some(ts), Some(zs)) = (item.targets.first(), item.target_zcc.first()) else { return 0 };
    ts.iter()
        .zip(zs.iter())
        .filter(|(t, z)| matches!(t, Target::Obj(o) if game.obj(*o).zone == Zone::Hand && game.obj(*o).zcc == **z + 1))
        .count() as i32
}

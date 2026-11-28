//! Infernal Vessel: "When this creature dies, if it wasn't a Demon, return it to the battlefield under its owner's
//! control with two +1/+1 counters on it. It's a Demon in addition to its other types."

use super::*;
use crate::card::EffectDef;
use crate::state::{Target, Zone};

pub const HANDLERS: &[Handler] = &[
    Handler::condition("InfernalVessel.InfernalVesselCondition", not_a_demon),
    Handler::effect("InfernalVessel.InfernalVesselReturnEffect", apply),
];

/// the permanent that died wasn't a Demon (its printed types, or the Demon this effect made it while it was there)
fn not_a_demon(game: &Game, _you: Seat, src: ObjId) -> bool {
    let o = game.obj(src);
    let was = o.lki_def.as_ref().unwrap_or(&o.def).subtypes.iter().any(|s| s == "Demon")
        || game.subtypes_while_there.iter().any(|(x, zcc, t)| *x == src && *zcc + 1 == o.zcc && t == "Demon");
    !was
}

/// back from the graveyard (wherever it went since, so long as it's in one) with two +1/+1 counters on it as it
/// enters (`ReturnFromGraveyardToBattlefieldWithCounterTargetEffect`'s `setEnterWithCounters`: "another creature
/// with power 2 or less enters" doesn't see a 2/1), then a Demon for as long as it stays
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.source_id();
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let back =
        EffectDef::ReturnFromGraveyardToBattlefieldTargetEffect { tapped: false, owner: true, counters: None, attacking: false };
    let zcc = game.obj(card).zcc;
    game.enter_counters.push((card, zcc, 2));
    cx.run_on(game, d, &[back], vec![Target::Obj(card)]);
    game.enter_counters.retain(|(o, z, _)| !(*o == card && *z == zcc));
    if game.obj(card).zone == Zone::Battlefield {
        let zcc = game.obj(card).zcc;
        game.subtypes_while_there.push((card, zcc, "Demon".into()));
    }
    Some(())
}

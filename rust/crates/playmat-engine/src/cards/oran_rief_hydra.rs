//! Oran-Rief Hydra: "Trample. Landfall — Whenever a land you control enters, put a +1/+1 counter on Oran-Rief Hydra.
//! If that land is a Forest, put two +1/+1 counters on Oran-Rief Hydra instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "OranRiefHydra.OranRiefHydraTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility(Zone.BATTLEFIELD, new OranRiefHydraEffect(), StaticFilters.FILTER_CONTROLLED_PERMANENT_LAND, false, SetTargetPointer.PERMANENT)",
        any,
    ),
    Handler::effect("OranRiefHydra.OranRiefHydraEffect", apply),
];

/// `checkTrigger`: a land its controller controls entered (the base's filter), the pointer on that land
fn any(_: &Game, _: &StackItem) -> bool {
    true
}

/// The land must still be on the battlefield (its `FixedTarget`) and the Hydra too; a Forest gives two counters
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((Target::Obj(land), zcc)) = cx.item.targets.first().zip(cx.item.target_zcc.first()).and_then(|(ts, zs)| {
        ts.first().copied().zip(zs.first().copied())
    }) else {
        return Some(());
    };
    let l = game.obj(land);
    if l.zone != Zone::Battlefield || l.zcc != zcc {
        return Some(());
    }
    let hydra = cx.source_id();
    if game.obj(hydra).zone != Zone::Battlefield {
        return Some(());
    }
    let n = if game.has_subtype(land, "Forest") { 2 } else { 1 };
    put_counters(game, hydra, "+1/+1", n, cx.controller());
    Some(())
}

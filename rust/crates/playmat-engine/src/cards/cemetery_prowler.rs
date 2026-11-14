//! Cemetery Prowler: "Whenever Cemetery Prowler enters or attacks, exile a card from a graveyard. Spells you cast
//! cost {1} less to cast for each card type they share with cards exiled with Cemetery Prowler."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("CemeteryProwler.CemeteryProwlerExileEffect", exile),
    Handler::cost("CemeteryProwler.CemeteryProwlerCostReductionEffect", CostKind::Reduce, your_spell, reduce),
];

/// a card from any graveyard (`TargetCardInGraveyard`, not targeted, `Player.choose`), exiled with this
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let can: Vec<ObjId> = (0..game.seats()).flat_map(|p| game.player(p).graveyard.clone()).collect();
    if can.is_empty() {
        return Some(());
    }
    let Some(card) = choose_cards(game, d, you, &can, &can, 1, 1, "exile")?.first().copied() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        game.obj_mut(card).exiled_with = Some(cx.source);
    }
    Some(())
}

fn your_spell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller
}

/// {1} less for each of the spell's card types among the cards in this permanent's exile zone
fn reduce(game: &Game, cx: &StaticCx, of: &CostCx, cost: &mut ManaCost) {
    let zone = (cx.source, game.obj(cx.source).zcc);
    let exiled: Vec<ObjId> = game.exile.iter().copied().filter(|c| game.obj(*c).exiled_with == Some(zone)).collect();
    let shared = of
        .face
        .types
        .iter()
        .filter(|t| exiled.iter().any(|c| game.obj(*c).def.types.contains(t)))
        .count() as u32;
    cost.generic = cost.generic.saturating_sub(shared);
}

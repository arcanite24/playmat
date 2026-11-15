//! The Kenriths' Royal Funeral: "When The Kenriths' Royal Funeral enters, exile up to two target legendary creature
//! cards from your graveyard. You draw X cards and you lose X life, where X is the greatest mana value among cards
//! exiled this way. Legendary spells you cast cost {1} less to cast for each card exiled with The Kenriths' Royal
//! Funeral."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TheKenrithsRoyalFuneral.TheKenrithsRoyalFuneralExileEffect", exile),
    Handler::cost("TheKenrithsRoyalFuneral.TheKenrithsRoyalFuneralCostEffect", CostKind::Reduce, legendary, reduce),
];

/// The targets still in your graveyard exiled into this permanent's exile zone; X their greatest mana value, then
/// you draw X and lose X life (nothing at all when none is left)
fn exile(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if cards.is_empty() {
        return Some(());
    }
    let x = cards.iter().map(|c| game.obj(*c).def.mana_value() as i32).max().unwrap_or(0);
    for c in cards {
        game.move_to(c, Zone::Exile);
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).exiled_with = Some(cx.source);
        }
    }
    draw(game, you, x.max(0) as usize);
    lose_life(game, you, x);
    Some(())
}

/// a legendary spell you cast (the card's, while what's playable is checked)
fn legendary(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller && of.face.supertypes.iter().any(|s| s == "LEGENDARY")
}

/// {1} less for each card in this permanent's exile zone
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let zone = (cx.source, game.obj(cx.source).zcc);
    let n = game.exile.iter().filter(|c| game.obj(**c).exiled_with == Some(zone)).count() as u32;
    cost.generic = cost.generic.saturating_sub(n);
}

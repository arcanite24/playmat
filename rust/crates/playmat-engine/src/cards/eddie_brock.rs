//! Eddie Brock // Venom, Lethal Protector: "Whenever Venom attacks, you may sacrifice another creature. If you do,
//! draw X cards, then you may put a permanent card with mana value X or less from your hand onto the battlefield,
//! where X is the sacrificed creature's mana value."

use super::*;
use crate::card::{CostDef, Filter};

pub const HANDLERS: &[Handler] = &[Handler::effect("EddieBrock.VenomLethalProtectorEffect", apply)
    .filters(&["StaticFilters.FILTER_ANOTHER_CREATURE"])];

/// `SacrificeTargetCost(another creature)`: `canPay`, "Sacrifice another creature?", the payment; then X cards drawn
/// and `PutCardFromHandOntoBattlefieldEffect` of a permanent card with mana value X or less
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cost = CostDef::SacrificeTargetCost { filter: cx.filters.first()?.clone(), amount: 1 };
    let src = cx.source_id();
    if !can_pay_cost(game, you, src, &cost) || !choose_use(game, d, you, "Sacrifice another creature?")? {
        return Some(());
    }
    let before: Vec<(ObjId, u32)> = game.battlefield.iter().map(|o| (*o, game.obj(*o).def.mana_value())).collect();
    if !pay_cost(game, d, you, src, &cost) {
        return if d.stopped() { None } else { Some(()) };
    }
    let x: u32 = before
        .iter()
        .filter(|(o, _)| game.obj(*o).zone != Zone::Battlefield)
        .map(|(_, mv)| *mv)
        .sum();
    draw(game, you, x as usize);
    let filter: Filter = serde_json::from_value(serde_json::json!([
        {"p": "permanent_card"},
        {"p": "mv", "cmp": "FEWER_THAN", "n": x + 1}
    ]))
    .ok()?;
    let e = EffectDef::PutCardFromHandOntoBattlefieldEffect { filter, tapped: false, attacking: false };
    cx.run(game, d, &[e]);
    Some(())
}

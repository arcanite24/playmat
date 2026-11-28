//! Heart-Piercer Manticore: "When Heart-Piercer Manticore enters, you may sacrifice another creature. When you do,
//! Heart-Piercer Manticore deals damage equal to that creature's power to any target."

use super::*;
use crate::card::{CostDef, TargetDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("HeartPiercerManticore.HeartPiercerManticoreEffect", apply)
    .filters(&["StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE"])];

/// `SacrificeTargetCost`: `canPay`, "Sacrifice another creature?", the payment; then a reflexive trigger dealing the
/// sacrificed creature's power to any target waits to go on the stack
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cost = CostDef::SacrificeTargetCost { filter: cx.filters.first()?.clone(), amount: 1 };
    let src = cx.source_id();
    if !can_pay_cost(game, you, src, &cost) || !choose_use(game, d, you, "Sacrifice another creature?")? {
        return Some(());
    }
    let before: Vec<(ObjId, i32)> = game.battlefield.iter().map(|o| (*o, game.power_toughness(*o).0)).collect();
    if !pay_cost(game, d, you, src, &cost) {
        return if d.stopped() { None } else { Some(()) };
    }
    let power: i32 =
        before.iter().filter(|(o, _)| game.obj(*o).zone != Zone::Battlefield).map(|(_, p)| *p).sum();
    let delayed = Delayed {
        // (`getRule` adds a '.' after the text's own)
        rule: "{this} deals damage equal to that creature's power to any target..".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::DamageTargetEffect { amount: Amount::Fixed(power) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetAnyTarget {
            min: 1,
            max: 1,
            another: false,
            not_first: false,
            distribute: None,
            x_count: false,
            count: None,
        }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    Some(())
}

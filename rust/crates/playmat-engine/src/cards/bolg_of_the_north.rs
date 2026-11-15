//! Bolg of the North: "When Bolg enters, you may sacrifice another creature. When you do, Bolg deals damage equal
//! to that creature's power to another target creature. If excess damage was dealt this way, amass Goblins X, where
//! X is that excess damage."

use super::*;
use crate::card::{CostDef, TargetDef};

const DAMAGE: &str = "BolgOfTheNorth.BolgOfTheNorthEffect";

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BolgOfTheNorth.BolgOfTheNorthSacrificeEffect", apply)
        .effects(&["new AmassEffect(1, SubType.GOBLIN)"])
        .filters(&["StaticFilters.FILTER_ANOTHER_CREATURE", "StaticFilters.FILTER_ANOTHER_TARGET_CREATURE"]),
    Handler::effect(DAMAGE, damage),
];

/// `SacrificeTargetCost`: `canPay`, "Sacrifice another creature?", the payment; then a reflexive trigger (another
/// target creature) carrying the sacrificed creature's power as it last was (`BolgOfTheNorthEffect.amount`)
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
    let mut effect: EffectDef = serde_json::from_value(
        serde_json::json!({"class": "CardEffect", "handler": DAMAGE, "args": [power.to_string()]}),
    )
    .ok()?;
    if let EffectDef::CardEffect { effects, .. } = &mut effect {
        *effects = vec![cx.effects.first()?.clone()];
    }
    let delayed = Delayed {
        rule: "{this} deals damage equal to that creature's power to another target creature. If excess damage was \
               dealt this way, amass Goblins X, where X is that excess damage."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetPermanent {
            filter: cx.filters.get(1)?.clone(),
            min: 1,
            max: 1,
            distribute: None,
            x_count: false,
            count: None,
        }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(src, you) });
    Some(())
}

/// The target is dealt the remembered amount (all of it, `Permanent.damage`); past its lethal damage as it was
/// before, amass Goblins that excess
fn damage(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    if game.obj(target).zone != Zone::Battlefield {
        return None;
    }
    let amount: i32 = cx.args.first()?.parse().ok()?;
    let src = cx.source_id();
    let lethal = super::damage_with_excess_effect::lethal_from(game, target, src);
    let dealt = deal_damage(game, src, Target::Obj(target), amount, false);
    let excess = (dealt - lethal).max(0);
    if excess > 0 {
        super::widespread_brutality::amass(game, d, cx, excess)?;
    }
    Some(())
}

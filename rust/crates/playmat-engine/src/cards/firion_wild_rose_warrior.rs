//! Firion, Wild Rose Warrior: "Whenever a nontoken Equipment you control enters, create a token that's a copy of it,
//! except it has "This Equipment's equip abilities cost {2} less to activate." Sacrifice that token at the beginning
//! of the next upkeep."

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("FirionWildRoseWarrior.FirionWildRoseWarriorEffect", apply),
    Handler::cost(
        "FirionWildRoseWarrior.FirionWildRoseWarriorReductionEffect",
        CostKind::Reduce,
        own_equip,
        two_less,
    ),
];

/// The Equipment that entered (`getValue("permanentEnteringBattlefield")`, copied as it is or last was): a token
/// copy with the static reduction added, sacrificed at the beginning of the next upkeep (`removeTokensCreatedAt(…,
/// UPKEEP, ANY)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.item.entering else { return Some(()) };
    let reduction: AbilityDef = serde_json::from_value(serde_json::json!({
        "class": "SimpleStaticAbility",
        "effects": [{"class": "CardStatic", "handler": "FirionWildRoseWarrior.FirionWildRoseWarriorReductionEffect"}],
        "rule": "This Equipment's equip abilities cost {2} less to activate.",
    }))
    .ok()?;
    let mods = move |def: &mut CardDef| def.abilities.push(reduction.clone());
    let made = token_copy_modified(game, d, of, you, 1, &mods);
    if d.stopped() {
        return None;
    }
    let delayed = Delayed {
        rule: "Sacrifice the token copies at the beginning of the next turn's upkeep.".into(),
        sacrifice: made.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_upkeep.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// an equip ability of the token itself (`source.getSourceId()` is the token's)
fn own_equip(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.card == cx.source && of.ability.is_some_and(|a| matches!(a, AbilityDef::EquipAbility { .. }))
}

/// `CardUtil.reduceCost(ability, 2)`: two generic
fn two_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}

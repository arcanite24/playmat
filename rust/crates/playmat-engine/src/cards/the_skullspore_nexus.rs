//! The Skullspore Nexus: "This spell costs {X} less to cast, where X is the greatest power among creatures you
//! control. Whenever one or more nontoken creatures you control die, create a green Fungus Dinosaur creature token
//! with base power and toughness each equal to the total power of those creatures. {2}, {T}: Double target
//! creature's power until end of turn."

use super::*;
use crate::card::{Amount, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::cost("TheSkullsporeNexus.TheSkullsporeNexusReductionEffect", CostKind::Reduce, applies, reduce)
        .in_any_zone(),
    Handler::trigger(
        "TheSkullsporeNexus.TheSkullsporeNexusTrigger",
        "new ZoneChangeBatchTriggeredAbility(new TheSkullsporeNexusTokenEffect(), Zone.BATTLEFIELD, Zone.GRAVEYARD, StaticFilters.FILTER_CONTROLLED_CREATURE_NON_TOKEN, TargetController.YOU, false)",
        any_batch,
    )
    .stack_info(total_power),
    // (the trigger's `CreateTokenEffect(new FungusDinosaurToken(amount))`, made as it triggers)
    Handler::effect("TheSkullsporeNexus.TheSkullsporeNexusTokenEffect", token).tokens(&["FungusDinosaurToken"]),
    Handler::effect("TheSkullsporeNexus.TheSkullsporeNexusDoubleEffect", double)
        .effects(&["new BoostTargetEffect(0, 0, Duration.EndOfTurn)"]),
];

/// its own spell
fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.card == cx.source
}

/// `GreatestAmongPermanentsValue.POWER_CONTROLLED_CREATURES`, as generic mana (`CardUtil.reduceCost`)
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let greatest = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == cx.controller && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .max()
        .unwrap_or(0);
    cost.generic = cost.generic.saturating_sub(greatest.max(0) as u32);
}

/// the dead creatures' total power as they last were on the battlefield
fn total(game: &Game, item: &StackItem) -> i32 {
    item.batch.iter().map(|(o, _, _)| game.obj(*o).power_lki.unwrap_or(0)).sum()
}

/// `getRule`: the total, as it triggered
fn total_power(game: &Game, item: &StackItem) -> String {
    format!("<br><br><i>Total power: {}</i>", total(game, item))
}

fn token(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let total = total(game, cx.item);
    let mut t = cx.tokens[0].clone();
    t.power = total;
    t.toughness = total;
    cx.create_tokens(game, d, &t, 1);
    Some(())
}

/// +X/+0 until end of turn, X its power as this resolves
fn double(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let c = cx.obj(0)?;
    let power = game.power_toughness(c).0;
    let mut e = cx.effects[0].clone();
    if let EffectDef::BoostTargetEffect { power: p, .. } = &mut e {
        *p = Amount::Fixed(power);
    }
    cx.run(game, d, &[e]);
    Some(())
}

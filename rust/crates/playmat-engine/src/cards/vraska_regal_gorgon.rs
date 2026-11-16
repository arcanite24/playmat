//! Vraska, Regal Gorgon: "+2: Put a +1/+1 counter on up to one target creature. That creature gains menace until end
//! of turn. −3: Destroy target creature. −10: For each creature card in your graveyard, put a +1/+1 counter on each
//! creature you control."

use super::*;
use crate::card::CounterDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("VraskaRegalGorgon.VraskaRegalGorgonEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURE", "StaticFilters.FILTER_CONTROLLED_CREATURE"])];

/// the creature cards in your graveyard counted, then `AddCountersAllEffect(+1/+1 × count, creatures you control)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let count = game
        .player(you)
        .graveyard
        .iter()
        .filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), **c))
        .count() as i32;
    let e = EffectDef::AddCountersAllEffect {
        counter: CounterDef { name: "+1/+1".into(), amount: count },
        filter: cx.filters[1].clone(),
        amount: None,
    };
    cx.run(game, d, &[e]);
    Some(())
}

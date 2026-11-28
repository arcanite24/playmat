//! Scout for Survivors: "Return up to three target creature cards with total mana value 3 or less from your
//! graveyard to the battlefield. Put a +1/+1 counter on each of them."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "ScoutForSurvivors.ScoutForSurvivorsTarget",
        "new TargetCardInYourGraveyard(0, 3, StaticFilters.FILTER_CARD_CREATURE, false)",
        can_target,
    ),
    Handler::effect("ScoutForSurvivors.ScoutForSurvivorsEffect", apply)
        .effects(&["new ReturnFromGraveyardToBattlefieldTargetEffect()"]),
];

/// `CardUtil.checkCanTargetTotalValueLimit(…, getManaValue, 3)`
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    super::joshua_phoenixs_dominant::within_total_mana_value(game, id, 3)
}

/// The targets still in the graveyard onto the battlefield together (`moveCards`), then a +1/+1 counter on each
/// that's a permanent now, in target order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if cards.is_empty() {
        return Some(());
    }
    cx.run_on(game, d, cx.effects, cards.iter().map(|c| Target::Obj(*c)).collect());
    if d.stopped() {
        return None;
    }
    for c in cards {
        if game.obj(c).zone == Zone::Battlefield {
            put_counters(game, c, "+1/+1", 1, cx.controller());
        }
    }
    Some(())
}

//! Kambal, Profiteering Mayor: "Whenever one or more tokens your opponents control enter, for each of them, create
//! a tapped token that's a copy of it. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "KambalProfiteeringMayor.KambalProfiteeringMayorTriggeredAbility",
        "new ZoneChangeBatchTriggeredAbility(new KambalProfiteeringMayorEffect(), null, Zone.BATTLEFIELD, StaticFilters.FILTER_PERMANENT_TOKEN, TargetController.OPPONENT, false).setTriggersLimitEachTurn(1)",
        any_batch,
    ),
    Handler::effect("KambalProfiteeringMayor.KambalProfiteeringMayorEffect", apply)
        .effects(&["new CreateTokenCopyTargetEffect(null, null, false, 1, true, false)"]),
];

/// a tapped copy of each, in the order they entered (Java's `FixedTarget(tokenId)`, read with
/// `getPermanentOrLKIBattlefield`: one that has left is copied as it last was)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for (o, _, _) in cx.item.batch.clone() {
        cx.run_on(game, d, cx.effects, vec![Target::Obj(o)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

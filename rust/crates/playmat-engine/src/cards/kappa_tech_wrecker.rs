//! Kappa Tech-Wrecker: "Ninjutsu {1}{G}. Kappa Tech-Wrecker enters with a deathtouch counter on it. Whenever Kappa
//! Tech-Wrecker deals combat damage to a player, you may remove a deathtouch counter from it. When you do, exile
//! target artifact or enchantment that player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KappaTechWrecker.KappaTechWreckerEffect", apply)
    .effects(&[r#"new DoWhenCostPaid(new ReflexiveTriggeredAbility(new ExileTargetEffect(), false), new RemoveCountersSourceCost(CounterType.DEATHTOUCH.createInstance()), "Remove a deathtouch counter?")"#])
    .filters(&["StaticFilters.FILTER_PERMANENT_ARTIFACT_OR_ENCHANTMENT"])];

/// `DoWhenCostPaid` of the reflexive exile, its target an artifact or enchantment the damaged player controls
/// (`ControllerIdPredicate` of the trigger's player pointer: with two players, an opponent or you)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0) else { return Some(()) };
    let mut e = cx.effects.first()?.clone();
    if let EffectDef::DoWhenCostPaid { targets, .. } = &mut e {
        let mut filter = cx.filters.first()?.clone();
        let who = if player == cx.controller() { "you" } else { "opponent" };
        filter.0.push(Pred::Controller { who: who.into() });
        *targets = vec![TargetDef::TargetPermanent { filter, min: 1, max: 1, distribute: None, x_count: false, count: None }];
    }
    cx.run(game, d, &[e]);
    Some(())
}

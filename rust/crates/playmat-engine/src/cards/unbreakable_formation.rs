//! Unbreakable Formation's addendum: "If you cast this spell during your main phase, put a +1/+1 counter on each of
//! those creatures and they gain vigilance until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnbreakableFormation.UnbreakableFormationEffect", apply).effects(&[
    "new GainAbilityControlledEffect(VigilanceAbility.getInstance(), Duration.EndOfTurn, StaticFilters.FILTER_PERMANENT_CREATURE)",
    "new AddCountersAllEffect(CounterType.P1P1.createInstance(), StaticFilters.FILTER_CONTROLLED_CREATURE)",
])];

/// `AddendumCondition`: your turn, a main phase, and the spell itself resolving (not a copy); then vigilance for
/// your creatures, then the counters (Java's order)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = game.obj(cx.item.card);
    let addendum = game.active == Some(cx.controller())
        && game.step.is_some_and(|s| s.is_main())
        && spell.zone == Zone::Stack
        && !spell.token;
    if addendum {
        cx.run(game, d, cx.effects);
    }
    Some(())
}

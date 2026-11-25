//! Sylvan Awakening: "Until your next turn, all lands you control become 2/2 Elemental creatures with reach,
//! indestructible, and haste. They're still lands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SylvanAwakening.BecomesCreatureAllEffect", apply)
    .effects(&[
        r#"new BecomesCreatureTargetEffect(new CreatureToken(2,2,"2/2 Elemental creatures with reach, indestructible, and haste").withSubType(SubType.ELEMENTAL).withAbility(ReachAbility.getInstance()).withAbility(IndestructibleAbility.getInstance()).withAbility(HasteAbility.getInstance()),false,true,Duration.UntilYourNextTurn)"#,
    ])
    .filters(&["StaticFilters.FILTER_CONTROLLED_PERMANENT_LANDS"])];

/// `BecomesCreatureAllEffect(token, "lands", lands you control, Duration.UntilYourNextTurn, false)`: the lands you
/// control as it resolves (its `affectedObjectList`), each the engine's "it's still a land" `BecomesCreatureTargetEffect`
/// until your next turn, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let lands: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c))
        .map(Target::Obj)
        .collect();
    cx.run_on(game, d, &cx.effects[..1], lands);
    (!d.stopped()).then_some(())
}

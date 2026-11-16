//! Gideon, the Oathsworn: "Whenever you attack with two or more non-Gideon creatures, put a +1/+1 counter on each
//! of those creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "GideonTheOathsworn.GideonTheOathswornTriggeredAbility",
        "new AttacksWithCreaturesTriggeredAbility(Zone.BATTLEFIELD, new GideonTheOathswornEffect(), 2, StaticFilters.FILTER_PERMANENT_CREATURES, true)",
        two_non_gideons,
    ),
    Handler::effect("GideonTheOathsworn.GideonTheOathswornEffect", apply),
];

/// `checkTrigger` is only asked as it triggers: resolving, it holds
fn resolving(game: &Game, t: &StackItem) -> bool {
    game.resolving_item.as_ref().is_some_and(|r| r.card == t.card && r.ability == t.ability)
}

/// two or more of the attackers (the trigger's pointer) are non-Gideon creatures
fn two_non_gideons(game: &Game, t: &StackItem) -> bool {
    if resolving(game, t) {
        return true;
    }
    let attackers = t.targets.first().map(|g| g.as_slice()).unwrap_or(&[]);
    attackers.iter().filter(|a| matches!(a, Target::Obj(o) if !game.has_subtype(*o, "Gideon"))).count() >= 2
}

/// a +1/+1 counter on each of those non-Gideon creatures still on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield && !game.has_subtype(o, "Gideon") {
            add_counters_by(game, o, "+1/+1", 1, Some(you));
        }
    }
    Some(())
}

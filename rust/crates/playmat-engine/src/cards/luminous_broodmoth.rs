//! Luminous Broodmoth: "Whenever a creature you control without flying dies, return it to the battlefield under its
//! owner's control with a flying counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "LuminousBroodmoth.LuminousBroodmothTriggeredAbility",
        "new DiesCreatureTriggeredAbility(Zone.BATTLEFIELD, new LuminousBroodmothEffect(), false, StaticFilters.FILTER_CONTROLLED_CREATURE, SetTargetPointer.PERMANENT)",
        check,
    ),
    Handler::effect("LuminousBroodmoth.LuminousBroodmothEffect", apply),
];

/// "without flying": the card that died, as the trigger is put on the stack and as it resolves
fn check(game: &Game, t: &StackItem) -> bool {
    let Some(Target::Obj(c)) = t.targets.first().and_then(|g| g.first()).copied() else { return false };
    !game.has_ability(c, &AbilityDef::FlyingAbility)
}

/// The card, still in its graveyard, onto the battlefield under its owner's control with a flying counter
/// (`setEnterWithCounters`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for c in cx.objs(0) {
        if game.obj(c).zone == Zone::Graveyard && !game.obj(c).token {
            let owner = game.obj(c).owner;
            enter_battlefield(game, d, c, owner, false);
            if game.obj(c).zone == Zone::Battlefield {
                add_counters_by(game, c, "flying", 1, Some(cx.controller()));
            }
        }
    }
    Some(())
}

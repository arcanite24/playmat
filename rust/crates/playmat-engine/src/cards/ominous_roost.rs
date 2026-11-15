//! Ominous Roost: "When Ominous Roost enters or whenever you cast a spell from your graveyard, create a 1/1 blue Bird
//! creature token with flying and "This creature can block only creatures with flying.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "OminousRoost.OminousRoostTriggeredAbility",
    "new OrTriggeredAbility(Zone.BATTLEFIELD, new CreateTokenEffect(new OminousRoostBirdToken()), new EntersBattlefieldTriggeredAbility(null), new SpellCastControllerTriggeredAbility(Zone.BATTLEFIELD, null, StaticFilters.FILTER_SPELL_A, false, SetTargetPointer.NONE, Zone.GRAVEYARD))",
    always,
)];

/// `checkTrigger`: its own entering, or a spell you cast from your graveyard (both the base triggers' events)
fn always(_: &Game, _: &StackItem) -> bool {
    true
}

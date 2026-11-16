//! Subira, Tulzidi Caravanner: "Haste. {1}: Another target creature with power 2 or less can't be blocked this turn.
//! {1}{R}, {T}, Discard your hand: Until end of turn, whenever a creature you control with power 2 or less deals
//! combat damage to a player, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SubiraTulzidiCaravanner.SubiraTulzidiCaravannerAbility",
    "new DealsDamageToAPlayerAllTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_CONTROLLED_CREATURE, false, SetTargetPointer.PERMANENT, true)",
    power_2_or_less,
)];

/// combat damage to a player by a creature you control (the base trigger, its pointer at the creature) with power 2
/// or less as it triggers (`checkTrigger` only: not checked again as it resolves)
fn power_2_or_less(game: &Game, t: &StackItem) -> bool {
    if game.checking_item.is_none() {
        return true;
    }
    match t.targets.first().and_then(|g| g.first()) {
        Some(Target::Obj(o)) => game.obj(*o).zone == Zone::Battlefield && game.power_toughness(*o).0 <= 2,
        _ => false,
    }
}

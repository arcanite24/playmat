//! Sharp-Eyed Rookie: "Vigilance. Whenever a creature you control enters, if its power is greater than this
//! creature's power or its toughness is greater than this creature's toughness, put a +1/+1 counter on this creature
//! and investigate."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SharpEyedRookie.SharpEyedRookieTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(Zone.BATTLEFIELD, new AddCountersSourceEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_CONTROLLED_CREATURE, false)",
    bigger,
)];

/// `checkInterveningIfClause`: the source a creature, and the entering creature's power or toughness greater than
/// its own
fn bigger(game: &Game, item: &StackItem) -> bool {
    let Some(it) = item.entering else { return false };
    if !game.has_type(item.card, CardType::Creature) || !game.has_type(it, CardType::Creature) {
        return false;
    }
    let (p, t) = game.power_toughness(item.card);
    let (ip, it) = game.power_toughness(it);
    p < ip || t < it
}

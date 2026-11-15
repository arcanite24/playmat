//! Hulkling, Burgeoning Bruiser: "Vigilance. Whenever another creature you control enters, if it has greater power or
//! toughness than Hulkling, put a +1/+1 counter on Hulkling."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "HulklingBurgeoningBruiser.HulklingBurgeoningBruiserTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL)",
    greater,
)];

/// its `checkTrigger` and intervening "if": the creature that entered and this (each as it is, or as it last was on
/// the battlefield): the creature's power or toughness is greater than this one's
fn greater(game: &Game, item: &StackItem) -> bool {
    let Some(it) = item.entering else { return false };
    let (Some((p, t)), Some((sp, st))) = (pt(game, it), pt(game, item.card)) else { return false };
    !(p <= sp && t <= st)
}

/// power and toughness on the battlefield, else as it last was there
fn pt(game: &Game, id: ObjId) -> Option<(i32, i32)> {
    let o = game.obj(id);
    if o.zone == Zone::Battlefield {
        return Some(game.power_toughness(id));
    }
    Some((o.power_lki?, o.toughness_lki?))
}

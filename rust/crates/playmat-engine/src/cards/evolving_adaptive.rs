//! Evolving Adaptive: "Evolving Adaptive enters with an oil counter on it. Evolving Adaptive gets +1/+1 for each oil
//! counter on it. Whenever another creature you control enters, if that creature has greater power or toughness than
//! Evolving Adaptive, put an oil counter on Evolving Adaptive."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "EvolvingAdaptive.EvolvingAdaptiveTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(new AddCountersSourceEffect(CounterType.OIL.createInstance()), StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL)",
    greater,
)];

/// the creature that entered and this (each as it is, or as it last was on the battlefield): the creature's power or
/// toughness is greater than this one's
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

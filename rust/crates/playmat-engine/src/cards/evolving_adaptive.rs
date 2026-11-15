//! Evolving Adaptive: "Evolving Adaptive enters with an oil counter on it. Evolving Adaptive gets +1/+1 for each oil
//! counter on it. Whenever another creature you control enters, if that creature has greater power or toughness than
//! Evolving Adaptive, put an oil counter on Evolving Adaptive."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "EvolvingAdaptive.EvolvingAdaptiveTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(new AddCountersSourceEffect(CounterType.OIL.createInstance()), StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL)",
    greater,
)];

/// the creature that entered and this, both on the battlefield: the creature's power or toughness is greater than
/// this one's
fn greater(game: &Game, item: &StackItem) -> bool {
    let Some(it) = item.entering else { return false };
    if game.obj(it).zone != Zone::Battlefield || game.obj(item.card).zone != Zone::Battlefield {
        return false;
    }
    let (p, t) = game.power_toughness(it);
    let (sp, st) = game.power_toughness(item.card);
    !(p <= sp && t <= st)
}

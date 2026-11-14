//! Moonshadow: "Whenever one or more permanent cards are put into your graveyard from anywhere while this creature
//! has a -1/-1 counter on it, remove a -1/-1 counter from this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "Moonshadow.MoonshadowTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new RemoveCounterSourceEffect(CounterType.M1M1.createInstance()), null, Zone.GRAVEYARD, StaticFilters.FILTER_CARD_PERMANENT, TargetController.YOU, true)",
    has_counter,
)];

/// "while this creature has a -1/-1 counter on it" (`getSourcePermanentIfItStillExists`)
fn has_counter(game: &Game, t: &StackItem) -> bool {
    let o = game.obj(t.card);
    o.zone == Zone::Battlefield && t.ability.is_some_and(|(_, zcc)| zcc == o.zcc) && o.counter("-1/-1") > 0
}

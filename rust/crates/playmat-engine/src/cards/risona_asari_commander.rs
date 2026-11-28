//! Risona, Asari Commander: "Whenever Risona deals combat damage to a player, if it doesn't have an indestructible
//! counter on it, put an indestructible counter on it. Whenever combat damage is dealt to you, remove an
//! indestructible counter from Risona."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("RisonaAsariCommander.RisonaAsariCommanderCondition", no_counter),
    Handler::trigger(
        "RisonaAsariCommander.RisonaAsariCommanderTriggeredAbility",
        "new YouDealtCombatDamageTriggeredAbility(new RemoveCounterSourceEffect(CounterType.INDESTRUCTIBLE.createInstance()))",
        any_batch,
    ),
];

/// the source is still on the battlefield, with no indestructible counter
fn no_counter(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Battlefield && o.counter("indestructible") == 0
}

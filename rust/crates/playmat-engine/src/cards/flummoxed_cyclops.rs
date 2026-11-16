//! Flummoxed Cyclops: "Whenever two or more creatures your opponents control attack, Flummoxed Cyclops can't block
//! this combat."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "FlummoxedCyclops.FlummoxedCyclopsTriggeredAbility",
        "new AureliaTheLawAboveTriggeredAbility(new CantBlockSourceEffect(Duration.EndOfCombat), 2)",
        opponent_attacks,
    ),
    Handler::restriction("FlummoxedCyclops.CantBlockSourceEffect", is_source, &[Restrict::CantBlock(no_block)])
        .args(&["one_of:Duration.EndOfCombat"]),
];

/// `DECLARED_ATTACKERS` with two or more attackers (the base), by an opponent of the controller
fn opponent_attacks(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    game.active.is_some_and(|a| a != item.controller)
}

/// `CantBlockSourceEffect(Duration.EndOfCombat)`: the source can't block (resolved, it lasts to the turn's cleanup:
/// blocks only happen in combat, so only an additional combat phase would tell)
fn is_source(_: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
}

fn no_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}

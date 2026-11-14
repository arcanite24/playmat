//! Syr Vondam, Sunstar Exemplar: "Whenever another creature you control dies or is put into exile, put a +1/+1
//! counter on Syr Vondam and you gain 1 life. When Syr Vondam dies or is put into exile while its power is 4 or
//! greater, destroy up to one target nonland permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "SyrVondamSunstarExemplar.SyrVondamSunstarExemplarFirstTriggeredAbility",
        "new LeavesBattlefieldAllTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_ANOTHER_CREATURE_YOU_CONTROL)",
        another_dies_or_exiled,
    ),
    Handler::trigger(
        "SyrVondamSunstarExemplar.SyrVondamSunstarExemplarSecondTriggeredAbility",
        "new LeavesBattlefieldTriggeredAbility(new DestroyTargetEffect())",
        dies_or_exiled_big,
    ),
];

/// to a graveyard or exile
fn another_dies_or_exiled(game: &Game, item: &StackItem) -> bool {
    item.left.is_some_and(|o| matches!(game.obj(o).zone, Zone::Graveyard | Zone::Exile))
}

/// this, to a graveyard or exile, with power 4 or greater as it last was
fn dies_or_exiled_big(game: &Game, item: &StackItem) -> bool {
    matches!(game.obj(item.card).zone, Zone::Graveyard | Zone::Exile) && item.lki_power.unwrap_or(0) >= 4
}

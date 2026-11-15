//! Bioessence Hydra: "Trample. Bioessence Hydra enters the battlefield with a +1/+1 counter on it for each loyalty
//! counter on planeswalkers you control. Whenever one or more loyalty counters are put on planeswalkers you control,
//! put that many +1/+1 counters on Bioessence Hydra."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("BioessenceHydra.BioessenceHydraDynamicValue", loyalty_on_your_planeswalkers),
    Handler::trigger(
        "BioessenceHydra.BioessenceHydraTriggeredAbility",
        "new CountersAddedAllTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance(), GetXValue.instance), CounterType.LOYALTY, StaticFilters.FILTER_CONTROLLED_PERMANENT_PLANESWALKER, TargetController.ANY, false)",
        not_itself,
    ),
];

/// The loyalty counters on the planeswalkers you control, in battlefield order
fn loyalty_on_your_planeswalkers(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Planeswalker))
        .map(|o| game.obj(*o).counter("loyalty"))
        .sum()
}

/// `COUNTERS_ADDED` of loyalty counters on a planeswalker you control (the base: X is how many, its pointer the
/// planeswalker), not on this permanent itself
fn not_itself(_: &Game, t: &StackItem) -> bool {
    t.x > 0 && !t.targets.first().is_some_and(|ts| ts.contains(&Target::Obj(t.card)))
}

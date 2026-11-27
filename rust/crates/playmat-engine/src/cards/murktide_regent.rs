//! Murktide Regent: "Delve. Flying. Murktide Regent enters the battlefield with a +1/+1 counter on it for each
//! instant and sorcery card exiled with it. Whenever an instant or sorcery card leaves your graveyard, put a +1/+1
//! counter on Murktide Regent."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("MurktideRegent.MurktideRegentValue", delved_instants_sorceries),
    Handler::trigger(
        "MurktideRegent.MurktideRegentTriggeredAbility",
        "new CardsLeaveGraveyardTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), StaticFilters.FILTER_CARD_INSTANT_OR_SORCERY)",
        any_leave,
    ),
];

fn delved_instants_sorceries(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    game.obj(source)
        .delved_cards
        .iter()
        .filter(|c| {
            let def = &game.obj(**c).def;
            def.is(CardType::Instant) || def.is(CardType::Sorcery)
        })
        .count() as i32
}

fn any_leave(_: &Game, _: &StackItem) -> bool {
    true
}

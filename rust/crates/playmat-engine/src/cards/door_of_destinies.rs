//! Door of Destinies: "As Door of Destinies enters, choose a creature type. Whenever you cast a spell of the chosen
//! type, put a charge counter on Door of Destinies. Creatures you control of the chosen type get +1/+1 for each
//! charge counter on Door of Destinies."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "DoorOfDestinies.AddCounterAbility",
        "new SpellCastControllerTriggeredAbility(new AddCountersSourceEffect(CounterType.CHARGE.createInstance(1)), false)",
        of_chosen_type,
    ),
    Handler::continuous("DoorOfDestinies.BoostCreatureEffectEffect", yours_of_chosen_type, Layer::ModifyPt(charge)),
];

/// the spell cast has the creature type the Door chose (none chosen: no trigger)
fn of_chosen_type(game: &Game, t: &StackItem) -> bool {
    let Some(chosen) = game.obj(t.card).chosen_type.as_ref() else { return false };
    t.cast_spell.is_some_and(|s| game.has_subtype(s, chosen))
}

/// creatures you control of the chosen type
fn yours_of_chosen_type(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield
        && o.controller == cx.controller
        && game.has_type(id, CardType::Creature)
        && game.obj(cx.source).chosen_type.as_ref().is_some_and(|t| game.has_subtype(id, t))
}

/// +1/+1 for each charge counter on the Door
fn charge(game: &Game, cx: &StaticCx, _: ObjId) -> (i32, i32) {
    let n = game.obj(cx.source).counter("charge");
    (n, n)
}

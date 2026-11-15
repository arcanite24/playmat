//! Anje, Maid of Dishonor: "Whenever Anje and/or one or more other Vampires you control enter, create a Blood
//! token. This ability triggers only once each turn. {2}, Sacrifice another creature or a Blood token: Each opponent
//! loses 2 life and you gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "AnjeMaidOfDishonor.AnjeMaidOfDishonorTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility(new CreateTokenEffect(new BloodToken()), new FilterControlledPermanent(SubType.VAMPIRE)).setTriggersLimitEachTurn(1)",
        this_or_vampire,
    ),
    Handler::predicate("AnjeMaidOfDishonor.AnjeMaidOfDishonorPredicate", another_creature_or_blood),
];

/// a permanent entering under your control: this, or a Vampire (the base's filter)
fn this_or_vampire(game: &Game, item: &StackItem) -> bool {
    item.entering.is_some_and(|e| e == item.card || game.has_subtype(e, "Vampire"))
}

/// a Blood token, or a creature other than the source
fn another_creature_or_blood(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    if game.obj(id).token && game.has_subtype(id, "Blood") {
        return true;
    }
    Some(id) != source && game.has_type(id, CardType::Creature)
}

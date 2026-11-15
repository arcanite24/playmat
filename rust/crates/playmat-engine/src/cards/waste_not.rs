//! Waste Not: "Whenever an opponent discards a creature card, create a 2/2 black Zombie creature token. Whenever an
//! opponent discards a land card, add {B}{B}. Whenever an opponent discards a noncreature, nonland card, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "WasteNot.WasteNotCreatureTriggeredAbility",
        "new DiscardsACardOpponentTriggeredAbility(new CreateTokenEffect(new ZombieToken()), false)",
        creature,
    ),
    Handler::trigger(
        "WasteNot.WasteNotLandTriggeredAbility",
        "new DiscardsACardOpponentTriggeredAbility(new BasicManaEffect(Mana.BlackMana(2)), false)",
        land,
    ),
    Handler::trigger(
        "WasteNot.WasteNotOtherTriggeredAbility",
        "new DiscardsACardOpponentTriggeredAbility(new DrawCardSourceControllerEffect(1), false)",
        other,
    ),
];

/// `DISCARDED_CARD` by an opponent (the base), of a creature card (checked as it triggers, as Java's `checkTrigger`)
fn creature(game: &Game, t: &StackItem) -> bool {
    game.checking_item.is_none() || t.discarded.is_some_and(|c| game.has_type(c, CardType::Creature))
}

/// of a land card
fn land(game: &Game, t: &StackItem) -> bool {
    game.checking_item.is_none() || t.discarded.is_some_and(|c| game.has_type(c, CardType::Land))
}

/// of a card neither a land nor a creature
fn other(game: &Game, t: &StackItem) -> bool {
    game.checking_item.is_none() || t.discarded.is_some_and(|c| !game.has_type(c, CardType::Land) && !game.has_type(c, CardType::Creature))
}

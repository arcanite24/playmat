//! Duskmantle Guildmage: "{1}{U}{B}: Whenever a card is put into an opponent's graveyard from anywhere this turn,
//! that player loses 1 life. {2}{U}{B}: Target player mills two cards."
//! The delayed ability waits for `ZONE_CHANGE` to an opponent's graveyard; "that player" is the card's owner, the
//! only opponent in a duel.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DuskmantleGuildmage.CardPutIntoOpponentGraveThisTurn",
    "new PutCardIntoGraveFromAnywhereAllTriggeredAbility(new LoseLifeOpponentsEffect(1), false, TargetController.OPPONENT)",
    any_card,
)];

/// any card (`!card.isCopy()`: a copy never reaches a graveyard here)
fn any_card(_: &Game, _: &StackItem) -> bool {
    true
}

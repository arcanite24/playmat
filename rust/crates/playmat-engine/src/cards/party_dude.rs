//! Party Dude: "(Gain the next level as a sorcery to add its ability.) When this Class enters, each player creates a
//! Food token. {1}{G}: Level 2. Whenever an artifact an opponent controls is put into a graveyard from the
//! battlefield, draw a card. {4}{G}: Level 3. Whenever one or more of your opponents are attacked, up to one target
//! attacking creature gets +X/+X until end of turn, where X is the number of cards in your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "PartyDude.PartyDudeTriggeredAbility",
    "new AttacksPlayerWithCreaturesTriggeredAbility(new BoostTargetEffect(CardsInControllerHandCount.ANY, CardsInControllerHandCount.ANY), StaticFilters.FILTER_PERMANENT_CREATURES, SetTargetPointer.NONE)",
    opponent_attacked,
)];

/// `DEFENDER_ATTACKED` of an opponent (a player, not a planeswalker): in a duel, the attackers are yours
fn opponent_attacked(game: &Game, t: &StackItem) -> bool {
    game.active == Some(t.controller)
}

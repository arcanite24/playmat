//! Rabble Rousing: "Whenever you attack with one or more creatures, create that many 1/1 green and white Citizen
//! creature tokens. Then if you control ten or more creatures, you may play the exiled card without paying its mana
//! cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("RabbleRousing.RabbleRousingValue", attackers_of_yours)];

/// `AttacksWithCreaturesTriggeredAbility.VALUEKEY_NUMBER_ATTACKERS`: the creatures you attacked with (those still
/// attacking as it resolves, in the same combat)
fn attackers_of_yours(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    attackers(game).into_iter().filter(|a| game.obj(*a).controller == you).count() as i32
}

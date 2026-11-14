//! Thieves' Tools: "When Thieves' Tools enters, create a Treasure token. Equipped creature can't be blocked as long
//! as its power is 3 or less. Equip {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ThievesTools.ThievesToolsCondition", small_equipped)];

/// The source on the battlefield, attached to a permanent with power 3 or less
fn small_equipped(game: &Game, _you: Seat, source: ObjId) -> bool {
    if game.obj(source).zone != Zone::Battlefield {
        return false;
    }
    game.attached(source).is_some_and(|to| game.power_toughness(to).0 <= 3)
}

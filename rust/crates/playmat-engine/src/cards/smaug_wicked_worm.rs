//! Smaug, Wicked Worm: "Flying. When Smaug enters, create X tapped Treasure tokens, where X is the number of artifacts
//! your opponents control. Whenever you cast a spell, if mana from a Treasure was spent to cast it, you draw a card
//! and lose 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SmaugWickedWorm.SmaugWickedWormCondition", treasure_spent)];

/// `getValue("spellCast")` had mana from a Treasure spent to cast it (`ManaPaidSourceWatcher.getTreasurePaid`)
fn treasure_spent(game: &Game, _you: Seat, _source: ObjId) -> bool {
    let item = game.resolving_item.as_ref().or(game.checking_item.as_ref());
    item.and_then(|i| i.cast_spell).is_some_and(|s| game.obj(s).treasure_spent > 0)
}

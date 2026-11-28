//! `DiscardCostCardManaValue`: "the mana value of the discarded card" (Mercurial Chemister's "{R}, {T}, Discard a
//! card: … damage to target creature equal to the discarded card's mana value").

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("DiscardCostCardManaValue", discarded_mana_value)];

/// The mana values of the cards the ability's `DiscardTargetCost` discarded, added up (0 without one)
fn discarded_mana_value(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(item) = game.resolving_item.as_ref().or(game.checking_item.as_ref()) else { return 0 };
    item.cost_discarded.iter().map(|c| game.obj(*c).def.mana_value() as i32).sum()
}

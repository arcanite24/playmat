//! Garna, Bloodfist of Keld: "Whenever another creature you control dies, draw a card if it was attacking.
//! Otherwise, Garna, Bloodfist of Keld deals 1 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("GarnaBloodfistOfKeld.GarnaBloodfistOfKeldCondition", was_attacking)];

/// `getValue("creatureDied")`'s `isAttacking()`: the resolving trigger's creature left the battlefield as an attacker
/// in this combat (its combat group keeps what it was attacking once it's gone)
fn was_attacking(game: &Game, _you: Seat, _source: ObjId) -> bool {
    let item = game.resolving_item.as_ref().or(game.checking_item.as_ref());
    let Some((died, _)) = item.and_then(|i| i.died) else { return false };
    game.combat_groups.iter().any(|g| g.attacker == died && g.left.is_some())
}

//! `EnchantmentOrEnchantedPredicate`: "enchanted creature or enchantment creature" (Inspire Awe, Nyx Herald, Temple
//! Thief)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("EnchantmentOrEnchantedPredicate", enchantment_or_enchanted)];

/// a creature that's an enchantment, or has an Aura attached
fn enchantment_or_enchanted(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    if !game.has_type(id, CardType::Creature) {
        return false;
    }
    game.has_type(id, CardType::Enchantment) || game.attachments(id).any(|a| game.has_subtype(a, "Aura"))
}

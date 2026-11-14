//! Koll, the Forgemaster: "Whenever another nontoken creature you control dies, if it was enchanted or equipped,
//! return it to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("KollTheForgemaster.KollTheForgemasterEnchantedPredicate", enchanted),
    Handler::predicate("KollTheForgemaster.KollTheForgemasterEquippedPredicate", equipped),
];

/// `input.getAttachments()` mapped through `getPermanentOrLKIBattlefield`: what's attached to it, or (once it left the
/// battlefield) what was as it left
fn attachments(game: &Game, id: ObjId) -> Vec<ObjId> {
    let o = game.obj(id);
    if o.zone == Zone::Battlefield {
        game.attachments(id).collect()
    } else {
        o.lki_attachments.iter().map(|(a, _)| *a).collect()
    }
}

/// an enchantment attached to it (not only an Aura)
fn enchanted(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    attachments(game, id).into_iter().any(|a| game.has_type(a, CardType::Enchantment))
}

fn equipped(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    attachments(game, id).into_iter().any(|a| game.has_subtype(a, "Equipment"))
}

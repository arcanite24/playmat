//! `EnchantedBySourceControllerPredicate`: "enchanted by Auras you control" (Archon of the Wild Rose)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("EnchantedBySourceControllerPredicate", enchanted_by_yours)];

/// an Aura attached to it that `you` control
fn enchanted_by_yours(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.attachments(id).any(|a| game.has_subtype(a, "Aura") && game.obj(a).controller == you)
}

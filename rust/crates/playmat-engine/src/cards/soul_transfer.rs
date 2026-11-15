//! Soul Transfer: "Choose one. If you control an artifact and an enchantment as you cast this spell, you may choose
//! both."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SoulTransfer.SoulTransferCondition", artifact_and_enchantment)];

fn artifact_and_enchantment(game: &Game, you: Seat, _source: ObjId) -> bool {
    let mine = |t: CardType| game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_type(*o, t));
    mine(CardType::Artifact) && mine(CardType::Enchantment)
}

//! Frozen in Ice: "Enchant creature. When this Aura enters, tap enchanted creature. Enchanted creature loses all
//! abilities and can't become untapped." (Its loss of abilities is `LoseAllAbilitiesAttachedEffect`, transpile.py.)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::rule("FrozenInIce.FrozenInIceUntapEffect", Rule::CantUntap(enchanted))];

/// `UNTAP` of the permanent the Aura is attached to, replaced with nothing
fn enchanted(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.attached(cx.source) == Some(id)
}

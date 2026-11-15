//! Draconic Intervention: "As an additional cost to cast this spell, exile an instant or sorcery card from your
//! graveyard. Draconic Intervention deals X damage to each non-Dragon creature, where X is the exiled card's mana
//! value. If a creature dealt damage this way would die this turn, exile it instead. Exile Draconic Intervention."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("DraconicIntervention.DraconicInterventionValue", exiled_mv)];

/// The mana value of the first card the spell's `ExileFromGraveCost` exiled (`getExiledCards().findFirst()`), 0
/// without one
fn exiled_mv(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    game.resolving_item
        .as_ref()
        .and_then(|i| i.cost_exiled.first())
        .map_or(0, |c| game.obj(*c).def.mana_value() as i32)
}

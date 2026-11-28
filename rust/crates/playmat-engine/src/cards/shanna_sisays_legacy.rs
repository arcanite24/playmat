//! Shanna, Sisay's Legacy: "Shanna can't be the target of abilities your opponents control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("ShannaSisaysLegacy.ShannaSisaysLegacyEffect", Rule::CantBeTargeted(opponents_ability))];

/// `TARGET` of Shanna herself by an ability (a stack object that isn't a spell) its controller doesn't control. An
/// ability's source is a permanent (or an emblem), or has an ability of its own on the stack; a card anywhere else,
/// on the stack as a spell or checked as one that could be cast from hand, is a spell
fn opponents_ability(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, src: ObjId) -> bool {
    let zone = game.obj(src).zone;
    let spell = zone == Zone::Stack && game.stack.iter().any(|i| i.card == src && i.ability.is_none());
    let ability = !spell
        && (matches!(zone, Zone::Battlefield | Zone::Command)
            || game.stack.iter().any(|i| i.card == src && i.ability.is_some()));
    ability && seat != cx.controller && id == cx.source
}

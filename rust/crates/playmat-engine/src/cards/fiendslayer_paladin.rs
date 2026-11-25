//! Fiendslayer Paladin: "First strike, lifelink. Fiendslayer Paladin can't be the target of black or red spells your
//! opponents control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("FiendslayerPaladin.FiendslayerPaladinEffect", Rule::CantBeTargeted(black_or_red_spell))];

/// `TARGET` of the Paladin itself by a black or red spell on the stack that's an instant an opponent controls, or
/// any sorcery: Java's `!isControlledBy(you) && isInstant || isSorcery` (so your own black or red sorceries can't
/// target it either)
fn black_or_red_spell(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, src: ObjId) -> bool {
    let spell = game.obj(src).zone == Zone::Stack && game.stack.iter().any(|i| i.card == src && i.ability.is_none());
    spell
        && id == cx.source
        && game.colors(src).iter().any(|c| matches!(c, Color::B | Color::R))
        && ((seat != cx.controller && game.has_type(src, CardType::Instant)) || game.has_type(src, CardType::Sorcery))
}

//! Sundering Archaic: "Converge — When Sundering Archaic enters, exile target nonland permanent an opponent controls
//! with mana value less than or equal to the number of colors of mana spent to cast Sundering Archaic. {2}: Put
//! target card from a graveyard on the bottom of its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("SunderingArchaic.SunderingArchaicPredicate", converge)];

/// its mana value is at most the colors of mana spent to cast the source (`ColorsOfManaSpentToCastCount`)
fn converge(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let colors = source.map_or(0, |s| game.obj(s).spent_colors.iter().filter(|n| **n > 0).count() as i32);
    colors >= game.obj(id).def.mana_value() as i32
}

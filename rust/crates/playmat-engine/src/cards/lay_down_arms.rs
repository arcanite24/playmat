//! Lay Down Arms: "Exile target creature with mana value less than or equal to the number of Plains you control. Its
//! controller gains 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("LayDownArms.LayDownArmsPredicate", at_most_plains)];

/// its mana value is at most the number of Plains you control
fn at_most_plains(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let plains = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Plains"))
        .count() as u32;
    game.obj(id).def.mana_value() <= plains
}

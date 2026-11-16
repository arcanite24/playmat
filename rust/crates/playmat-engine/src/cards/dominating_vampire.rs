//! Dominating Vampire: "When this creature enters, gain control of target creature with mana value less than or equal
//! to the number of Vampires you control until end of turn. Untap that creature. It gains haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("DominatingVampire.DominatingVampirePredicate", at_most_vampires)];

/// its mana value is at most the number of Vampires you control
fn at_most_vampires(game: &Game, you: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let vampires = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Vampire"))
        .count() as u32;
    game.obj(id).def.mana_value() <= vampires
}

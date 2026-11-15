//! Howling Galefang: "Vigilance. Howling Galefang has haste as long as you own a card in exile that has an
//! Adventure."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("HowlingGalefang.HowlingGalefangCondition", adventure_in_exile)];

/// a card you own in exile is an `AdventureCard` (an Omen isn't)
fn adventure_in_exile(game: &Game, you: Seat, _: ObjId) -> bool {
    game.exile.iter().any(|c| {
        let o = game.obj(*c);
        o.owner == you && o.def.adventure.is_some() && !o.def.omen
    })
}

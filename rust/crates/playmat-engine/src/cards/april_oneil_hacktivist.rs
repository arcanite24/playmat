//! April O'Neil, Hacktivist: "At the beginning of your end step, draw a card for each card type among spells you've
//! cast this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("AprilONeilHacktivist.AprilONeilHacktivistValue", card_types_cast)];

/// `AprilONeilHacktivistWatcher`: the different card types among the spells you cast this turn (each card's types)
fn card_types_cast(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let mut types: Vec<CardType> = Vec::new();
    for s in game.watch.spells.iter().filter(|s| s.caster == you) {
        for t in &game.obj(s.card).def.types {
            if !types.contains(t) {
                types.push(*t);
            }
        }
    }
    types.len() as i32
}

//! Drag to the Bottom: "Domain — All creatures get -X/-X until end of turn, where X is 1 plus the number of basic
//! land types among lands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("DragToTheBottom.DragToTheBottomValue", minus_domain)];

/// -(1 + `DomainValue.REGULAR`)
fn minus_domain(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    let domain = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
        .iter()
        .filter(|t| {
            game.battlefield.iter().any(|b| {
                let o = game.obj(*b);
                o.controller == you && o.is(CardType::Land) && game.has_subtype(*b, t)
            })
        })
        .count() as i32;
    -(1 + domain)
}

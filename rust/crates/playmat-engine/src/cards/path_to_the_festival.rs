//! Path to the Festival: "Search your library for a basic land card, put that card onto the battlefield tapped, then
//! shuffle. Then if there are three or more basic land types among lands you control, scry 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("PathToTheFestival.PathToTheFestivalCondition", domain_three)];

/// `DomainValue.REGULAR >= 3`: basic land types among lands you control
fn domain_three(game: &Game, you: Seat, _source: ObjId) -> bool {
    ["Plains", "Island", "Swamp", "Mountain", "Forest"]
        .iter()
        .filter(|t| {
            game.battlefield.iter().any(|b| {
                let o = game.obj(*b);
                o.controller == you && o.is(CardType::Land) && game.has_subtype(*b, t)
            })
        })
        .count()
        >= 3
}

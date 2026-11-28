//! Seven of Nine, Unassimilated: "… Then discard a card unless you control five or more differently named lands.
//! {6}{U}: … Activate only if you control seven or more differently named lands."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("SevenOfNineUnassimilated.SevenOfNineUnassimilatedCondition.FIVE_LANDS", five),
    Handler::condition("SevenOfNineUnassimilated.SevenOfNineUnassimilatedCondition.SEVEN_LANDS", seven),
];

/// `DifferentlyNamedPermanentCount(FILTER_CONTROLLED_PERMANENT_LANDS)`: lands you control, face-down ones (no name)
/// left out
fn lands(game: &Game, you: Seat) -> usize {
    let mut names: Vec<&str> = Vec::new();
    for b in game.battlefield.iter() {
        let o = game.obj(*b);
        if o.controller == you && !o.face_down && game.has_type(*b, CardType::Land) && !names.contains(&o.name()) {
            names.push(o.name());
        }
    }
    names.len()
}

fn five(game: &Game, you: Seat, _: ObjId) -> bool {
    lands(game, you) >= 5
}

fn seven(game: &Game, you: Seat, _: ObjId) -> bool {
    lands(game, you) >= 7
}

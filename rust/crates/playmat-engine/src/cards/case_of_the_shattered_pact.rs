//! Case of the Shattered Pact: "To solve — There are five colors among permanents you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("CaseOfTheShatteredPact.CaseOfTheShatteredPactCondition", five_colors)];

/// `ColorsAmongControlledPermanentsCount.ALL_PERMANENTS` is 5
fn five_colors(game: &Game, you: Seat, _: ObjId) -> bool {
    let mut seen: Vec<crate::card::Color> = Vec::new();
    for o in game.battlefield.iter().copied() {
        if game.obj(o).controller != you {
            continue;
        }
        for c in game.colors(o) {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
    }
    seen.len() >= 5
}

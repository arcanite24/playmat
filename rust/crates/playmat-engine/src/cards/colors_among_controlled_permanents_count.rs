//! `ColorsAmongControlledPermanentsCount.ALLIES` and `.OTHER_ARTIFACTS`: "the number of colors among Allies (other
//! artifacts) you control" (Earthen Ally, General Tazri, Karn, Gilded Guardian)

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("ColorsAmongControlledPermanentsCount.ALLIES", allies),
    Handler::value("ColorsAmongControlledPermanentsCount.OTHER_ARTIFACTS", other_artifacts),
];

/// the colors of the permanents you control that pass `keep`, counted once each
fn colors_among(game: &Game, you: Seat, keep: impl Fn(ObjId) -> bool) -> i32 {
    let mut seen: Vec<Color> = Vec::new();
    for b in game.battlefield.iter().copied() {
        if game.obj(b).controller != you || !keep(b) {
            continue;
        }
        for c in game.colors(b) {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
    }
    seen.len() as i32
}

fn allies(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    colors_among(game, you, |b| game.has_subtype(b, "Ally"))
}

fn other_artifacts(game: &Game, you: Seat, source: ObjId, _x: i32) -> i32 {
    colors_among(game, you, |b| b != source && game.has_type(b, CardType::Artifact))
}

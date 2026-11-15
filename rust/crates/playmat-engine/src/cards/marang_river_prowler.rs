//! Marang River Prowler: "Marang River Prowler can't block and can't be blocked. You may cast Marang River Prowler
//! from your graveyard as long as you control a black or green permanent."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("MarangRiverProwler.MarangRiverProwlerCastEffect", AsThough::Play(play)).in_any_zone()];

/// `CAST_FROM_NOT_OWN_HAND_ZONE`: this card, by its owner, from their graveyard, while they control a black or green
/// permanent
fn play(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _: &CardDef) -> Option<PlayWay> {
    let o = game.obj(id);
    let colored = game.battlefield.iter().any(|b| {
        game.obj(*b).controller == cx.controller && {
            let c = game.colors(*b);
            c.contains(&Color::B) || c.contains(&Color::G)
        }
    });
    (id == cx.source && o.owner == seat && o.zone == Zone::Graveyard && colored).then(PlayWay::default)
}

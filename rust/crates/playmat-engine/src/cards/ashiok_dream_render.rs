//! Ashiok, Dream Render: "Spells and abilities your opponents control can't cause their controller to search their
//! library. −1: Target player mills four cards. Then exile each opponent's graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("AshiokDreamRender.AshiokDreamRenderEffect", Rule::CantSearchLibrary(cant_search))];

/// the searcher controls the spell or ability, searches their own library, and is an opponent
fn cant_search(game: &Game, cx: &StaticCx, seat: Seat, owner: Seat, source: ObjId) -> bool {
    let controller = game.obj(source).controller;
    seat == controller && owner == controller && cx.opponent(controller)
}

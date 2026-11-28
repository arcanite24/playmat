//! Maha, Its Feathers Night: "Creatures your opponents control have base toughness 1."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("MahaItsFeathersNight.MahaItsFeathersNightEffect", affects, Layer::SetToughness(one))];

/// `StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE`
fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield && cx.opponent(o.controller) && game.has_type(id, CardType::Creature)
}

fn one(_: &Game, _: &StaticCx, _: ObjId) -> i32 {
    1
}

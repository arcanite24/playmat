//! Toxrill, the Corrosive: "Creatures you don't control get -1/-1 for each slime counter on them."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "ToxrillTheCorrosive.ToxrillTheCorrosiveEffect",
    affects,
    Layer::ModifyPt(minus),
)];

/// `FILTER_CREATURE_YOU_DONT_CONTROL`
fn affects(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield && o.controller != cx.controller && game.has_type(id, CardType::Creature)
}

fn minus(game: &Game, _: &StaticCx, id: ObjId) -> (i32, i32) {
    let n = game.obj(id).counter("slime");
    (-n, -n)
}

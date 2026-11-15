//! Timberpack Wolf: "Timberpack Wolf gets +1/+1 for each other creature you control named Timberpack Wolf."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "TimberpackWolf.TimberpackWolfEffect",
    itself,
    Layer::ModifyPt(other_wolves),
)];

fn itself(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// the creatures you control named Timberpack Wolf, less one
fn other_wolves(game: &Game, cx: &StaticCx, _: ObjId) -> (i32, i32) {
    let wolves = game
        .battlefield
        .iter()
        .filter(|o| {
            let x = game.obj(**o);
            x.controller == cx.controller && x.name() == "Timberpack Wolf" && game.has_type(**o, CardType::Creature)
        })
        .count() as i32;
    let n = (wolves - 1).max(0);
    (n, n)
}

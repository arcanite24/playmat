//! Wintermoor Commander: "Wintermoor Commander's toughness is equal to the number of Knights you control."
//! (`SetBaseToughnessSourceEffect(new PermanentsOnBattlefieldCount(Knights you control))`)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "WintermoorCommander.SetBaseToughnessSourceEffect",
    own,
    Layer::SetToughness(knights),
)
.args(&["one_of:xValue"])
.filters(&["new FilterControlledPermanent(SubType.KNIGHT)"])];

fn own(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// `SubLayer.CharacteristicDefining_7a`: the Knights its controller controls
fn knights(game: &Game, cx: &StaticCx, _: ObjId) -> i32 {
    game.battlefield.iter().filter(|o| cx.matches(game, 0, **o)).count() as i32
}

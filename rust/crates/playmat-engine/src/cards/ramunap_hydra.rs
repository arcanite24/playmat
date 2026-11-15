//! Ramunap Hydra: "Vigilance, reach, trample. Ramunap Hydra gets +1/+1 as long as you control a Desert. Ramunap
//! Hydra gets +1/+1 as long as there is a Desert card in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("RamunapHydra.RamunapHydraBoostEffect", itself_with_desert, Layer::ModifyPt(plus_one))
        .args(&["one_of:1", "one_of:1"])];

/// the Hydra, while a Desert card is in your graveyard (`CardsInControllerGraveyardCondition(1, filter)`)
fn itself_with_desert(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source && game.player(cx.controller).graveyard.iter().any(|c| game.has_subtype(*c, "Desert"))
}

fn plus_one(_: &Game, _: &StaticCx, _: ObjId) -> (i32, i32) {
    (1, 1)
}

//! Desperate Castaways: "Desperate Castaways can't attack unless you control an artifact."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "DesperateCastaways.DesperateCastawaysEffect",
    no_artifact,
    &[Restrict::CantAttack(cant)],
)];

/// the Castaways itself, while you control no artifact
fn no_artifact(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
        && !game
            .battlefield
            .iter()
            .any(|a| game.obj(*a).controller == cx.controller && game.has_type(*a, CardType::Artifact))
}

fn cant(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

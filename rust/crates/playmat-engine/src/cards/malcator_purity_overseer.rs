//! Malcator, Purity Overseer: "At the beginning of your end step, if three or more artifacts entered the battlefield
//! under your control this turn, create a 3/3 colorless Phyrexian Golem artifact creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("MalcatorPurityOverseer.MalcatorPurityOverseerCondition", three_artifacts),
    Handler::value("MalcatorPurityOverseer.MalcatorPurityOverseerValue", artifacts_entered),
];

/// the artifacts that entered under your control this turn (`MalcatorPurityOverseerWatcher`)
fn artifacts_entered(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.entered.iter().filter(|(c, artifact, _, _)| *c == you && *artifact).count() as i32
}

fn three_artifacts(game: &Game, you: Seat, source: ObjId) -> bool {
    artifacts_entered(game, you, source, 0) >= 3
}

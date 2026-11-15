//! Tower Worker: "Reach. {T}: Add {C}. If you control creatures named Mine Worker and Power Plant Worker, add {C}{C}{C}
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("TowerWorker.TowerWorkerValue", tron)];

fn tron(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let named = |n: &str| {
        game.battlefield.iter().any(|o| {
            game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && game.obj(*o).name() == n
        })
    };
    if named("Mine Worker") && named("Power Plant Worker") {
        3
    } else {
        1
    }
}

//! Mine Worker: "{T}: You gain 1 life. If you control creatures named Power Plant Worker and Tower Worker, you gain 3
//! life instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MineWorker.MineWorkerEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let named = |n: &str| {
        game.battlefield.iter().any(|o| {
            game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && game.obj(*o).name() == n
        })
    };
    let life = if named("Power Plant Worker") && named("Tower Worker") { 3 } else { 1 };
    gain_life(game, you, life);
    Some(())
}

//! Thornbow Archer: "Whenever Thornbow Archer attacks, each opponent who doesn't control an Elf loses 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThornbowArcher.ThornbowArcherEffect", apply)];

/// an Elf creature (`FilterCreaturePermanent` with the Elf subtype), opponents in turn order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let elf = game.battlefield.iter().any(|o| {
            game.obj(*o).controller == p && game.has_type(*o, CardType::Creature) && game.has_subtype(*o, "Elf")
        });
        if !elf {
            lose_life(game, p, 1);
        }
    }
    Some(())
}

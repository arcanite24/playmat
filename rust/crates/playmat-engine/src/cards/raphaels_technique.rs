//! Raphael's Technique: "Each player may discard their hand and draw seven cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RaphaelsTechnique.RaphaelsTechniqueEffect", apply)];

/// Each player from you in turn order is asked; then each who said yes, in that order, discards their hand and
/// draws seven
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut wheelers = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        if choose_use(game, d, p, "Discard your hand and draw seven?")? {
            wheelers.push(p);
        }
    }
    for p in wheelers {
        for c in game.player(p).hand.clone() {
            discard_card(game, p, c);
        }
        draw(game, p, 7);
    }
    Some(())
}

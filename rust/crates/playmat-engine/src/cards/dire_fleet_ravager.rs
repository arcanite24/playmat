//! Dire Fleet Ravager: "When Dire Fleet Ravager enters, each player loses a third of their life, rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DireFleetRavager.DireFleetRavagerEffect", apply)];

/// each player in turn order from you, `Math.ceil(life / 3.0)` (a third of a negative life rounds toward zero)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    for p in players {
        let life = game.player(p).life;
        let n = (life as f64 / 3.0).ceil() as i32;
        if n > 0 {
            lose_life(game, p, n);
        }
    }
    Some(())
}

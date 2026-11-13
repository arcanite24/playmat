//! Depopulate: "Each player who controls a multicolored creature draws a card. Then destroy all creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Depopulate.DepopulateEffect", apply)];

/// The players who control a multicolored creature each draw a card, in turn order from you
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let players: Vec<Seat> =
        (0..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let drawing: Vec<Seat> = players
        .into_iter()
        .filter(|p| {
            game.battlefield.iter().any(|o| {
                game.obj(*o).controller == *p && game.has_type(*o, CardType::Creature) && game.colors(*o).len() > 1
            })
        })
        .collect();
    for p in drawing {
        draw(game, p, 1);
    }
    Some(())
}

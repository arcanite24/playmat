//! Shatter the Sky: "Each player who controls a creature with power 4 or greater draws a card. Then destroy all
//! creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShatterTheSky.ShatterTheSkyEffect", apply)
    .effects(&["new DestroyAllEffect(StaticFilters.FILTER_PERMANENT_CREATURE)"])];

/// the controllers of the creatures with power 4 or more, in battlefield order, each draw once; then the board
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut drawers: Vec<Seat> = Vec::new();
    for o in game.battlefield.iter() {
        let c = game.obj(*o).controller;
        if game.has_type(*o, CardType::Creature) && game.power_toughness(*o).0 > 3 && !drawers.contains(&c) {
            drawers.push(c);
        }
    }
    for p in drawers {
        draw(game, p, 1);
    }
    cx.run(game, d, cx.effects);
    Some(())
}

//! Bushmeat Poacher: "{1}, {T}, Sacrifice another creature: You gain life equal to the sacrificed creature's
//! toughness. Draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BushmeatPoacher.BushmeatPoacherEffect", apply)];

/// the sacrificed creature's toughness as it last was on the battlefield (the cost's one creature)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let toughness = cx.item.sacrificed_toughness?;
    if toughness > 0 {
        gain_life(game, cx.controller(), toughness);
    }
    draw(game, cx.controller(), 1);
    Some(())
}

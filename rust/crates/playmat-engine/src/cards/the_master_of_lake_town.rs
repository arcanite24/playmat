//! The Master of Lake-town: "Deathtouch. Whenever a player loses life, that player mills that many cards. When The
//! Master of Lake-town dies, draw a card for each graveyard with seven or more cards in it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheMasterOfLakeTown.TheMasterOfLakeTownEffect", apply)];

/// the players in range whose graveyards have seven or more cards: that many cards drawn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let count = (0..game.seats()).filter(|p| !game.player(*p).lost && game.player(*p).graveyard.len() >= 7).count();
    if count > 0 {
        draw(game, you, count);
    }
    Some(())
}

//! `DrawCardsEqualToDifferenceEffect` (Iymrith, Desert Doom): "Then if you have fewer than three cards in hand,
//! draw cards equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DrawCardsEqualToDifferenceEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.arg_int(0)? - game.player(you).hand.len() as i32;
    if n > 0 {
        draw(game, you, n as usize);
    }
    Some(())
}

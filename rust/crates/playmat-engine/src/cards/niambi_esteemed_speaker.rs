//! Niambi, Esteemed Speaker: "Flash. When Niambi, Esteemed Speaker enters the battlefield, you may return another
//! target creature you control to its owner's hand. If you do, you gain life equal to that creature's mana value.
//! {1}{W}{U}, {T}, Discard a legendary card: Draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NiambiEsteemedSpeaker.NiambiEsteemedSpeakerEffect", apply)];

/// After the return: the creature's mana value (the card's, wherever it went), when above 0
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let mv = game.obj(o).def.mana_value() as i32;
    if mv > 0 {
        gain_life(game, cx.controller(), mv);
    }
    Some(())
}

//! Shreds of Sanity: "Return up to one target instant card and up to one target sorcery card from your graveyard to
//! your hand, then discard a card. Exile Shreds of Sanity."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShredsOfSanity.ShredsOfSanityEffect", apply)];

/// both cards to hand together, then `discardOne`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..2 {
        if let Some(c) = cx.obj(i).filter(|c| game.obj(*c).zone == Zone::Graveyard) {
            game.move_to(c, Zone::Hand);
        }
    }
    discard(game, d, you, 1);
    (!d.stopped()).then_some(())
}

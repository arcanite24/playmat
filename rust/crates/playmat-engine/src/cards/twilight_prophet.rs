//! Twilight Prophet: "At the beginning of your upkeep, if you have the city's blessing, reveal the top card of your
//! library and put it into your hand. Each opponent loses X life and you gain X life, where X is that card's mana
//! value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TwilightProphet.TwilightProphetEffect", apply)];

/// The top card is revealed and put into your hand; with a mana value above 0, each opponent loses that much life
/// (`LoseLifeOpponentsEffect`) and you gain it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = game.player(you).library.first().copied() else { return Some(()) };
    let (src, zcc) = (cx.source_id(), game.obj(cx.source_id()).zcc);
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: vec![(card, Zone::Library)] });
    game.move_to(card, Zone::Hand);
    let x = game.obj(card).def.mana_value() as i32;
    if x > 0 {
        for s in 0..game.seats() {
            if s != you {
                lose_life(game, s, x);
            }
        }
        gain_life(game, you, x);
    }
    Some(())
}

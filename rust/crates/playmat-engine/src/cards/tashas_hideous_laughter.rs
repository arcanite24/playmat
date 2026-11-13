//! Tasha's Hideous Laughter: "Each opponent exiles cards from the top of their library until that player has exiled
//! cards with total mana value 20 or more."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TashasHideousLaughter.TashasHideousLaughterEffect", apply)];

/// Each opponent's cards from the top until their mana values reach 20, then one move to exile for all of them
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        let mut total = 0;
        for c in game.player(p).library.clone() {
            cards.push(c);
            total += game.obj(c).def.mana_value();
            if total >= 20 {
                break;
            }
        }
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Exiled) {
        return None;
    }
    Some(())
}

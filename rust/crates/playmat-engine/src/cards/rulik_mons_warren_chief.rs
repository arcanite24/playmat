//! Rulik Mons, Warren Chief: "Menace. Whenever Rulik Mons attacks, look at the top card of your library. If it's a
//! land card, you may put it onto the battlefield tapped. If you didn't put a card onto the battlefield this way,
//! create a 1/1 red Goblin creature token."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RulikMonsWarrenChief.RulikMonsWarrenChiefEffect", apply).tokens(&["GoblinToken"])];

/// `lookAtCards`, the land's "Put … into play tapped?", else a Goblin
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut played = false;
    if let Some(&card) = game.player(you).library.first() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.looked_at.push(crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: vec![card] });
        if game.has_type(card, CardType::Land) {
            let message = format!("Put {} into play tapped?", game.obj(card).def.name);
            if choose_use(game, d, you, &message)? {
                if !put_cards(game, d, you, &[card], PutCards::BattlefieldTapped) {
                    return None;
                }
                played = game.obj(card).zone == Zone::Battlefield;
            }
        }
    }
    if !played {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

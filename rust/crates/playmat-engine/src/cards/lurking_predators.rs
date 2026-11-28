//! Lurking Predators: "Whenever an opponent casts a spell, reveal the top card of your library. If it's a creature
//! card, put it onto the battlefield. Otherwise, you may put that card on the bottom of your library."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("LurkingPredators.LurkingPredatorsEffect", apply)];

/// `revealCards` (titled by the source), then the creature card `moveCards` onto the battlefield, else "Put … on the
/// bottom of your library?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    if game.has_type(card, CardType::Creature) {
        if !put_cards(game, d, you, &[card], PutCards::Battlefield) {
            return None;
        }
    } else {
        let name = log::object(game, card);
        if choose_use(game, d, you, &format!("Put {name} on the bottom of your library?"))? {
            to_library(game, card, false);
        }
    }
    Some(())
}

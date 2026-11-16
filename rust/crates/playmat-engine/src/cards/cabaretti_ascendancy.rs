//! Cabaretti Ascendancy: "At the beginning of your upkeep, look at the top card of your library. If it's a creature
//! or planeswalker card, you may reveal it and put it into your hand. If you don't put the card into your hand, you
//! may put it on the bottom of your library."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("CabarettiAscendancy.CabarettiAscendencyEffect", apply)];

/// `lookAtCards` (titled by the source), the creature or planeswalker's "Reveal … and put it in your hand?", else
/// "Put … on the bottom of your library?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: vec![card] });
    let fits = game.has_type(card, CardType::Creature) || game.has_type(card, CardType::Planeswalker);
    let name = log::object(game, card);
    if fits && choose_use(game, d, you, &format!("Reveal {name} and put it in your hand?"))? {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        if !put_cards(game, d, you, &[card], PutCards::Hand) {
            return None;
        }
    } else if choose_use(game, d, you, &format!("Put {name} on the bottom of your library?"))? {
        to_library(game, card, false);
    }
    Some(())
}

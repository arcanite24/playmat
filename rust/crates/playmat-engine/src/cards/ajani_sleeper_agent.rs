//! Ajani, Sleeper Agent: "+1: Reveal the top card of your library. If it's a creature or planeswalker card, put it
//! into your hand. Otherwise, you may put it on the bottom of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AjaniSleeperAgent.AjaniSleeperAgentEffect", apply)];

/// the reveal (titled by the source), then the hand, or "Put … on the bottom of your library?"
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
    if game.has_type(card, CardType::Creature) || game.has_type(card, CardType::Planeswalker) {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Hand) {
            return None;
        }
    } else {
        let name = game.obj(card).def.name.clone();
        if choose_use(game, d, you, &format!("Put {name} on the bottom of your library?"))? {
            to_library(game, card, false);
        }
    }
    Some(())
}

//! Sword-Point Diplomacy: "Reveal the top three cards of your library. For each of those cards, put that card into
//! your hand unless any opponent pays 3 life. Then exile the rest."

use super::*;
use crate::card::{CostDef, PutCards};

pub const HANDLERS: &[Handler] = &[Handler::effect("SwordPointDiplomacy.SwordPointDiplomacyEffect", apply)];

/// The top three revealed; for each card in order, each opponent who can pay 3 life is asked "Pay 3 life to prevent
/// … from getting …?" (every opponent is asked, even after one paid); the cards no one paid for go to your hand
/// together, then the rest are exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: top.iter().map(|c| (*c, Zone::Library)).collect(),
    });
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    let (mut to_hand, mut rest) = (Vec::new(), Vec::new());
    for card in top {
        let mut keep = true;
        for &opp in &opponents {
            if game.player(opp).life < 3 {
                continue;
            }
            let message = format!("Pay 3 life to prevent {} from getting {}?", log::player(you), log::object(game, card));
            if choose_use(game, d, opp, &message)? && pay_cost(game, d, opp, cx.source_id(), &CostDef::PayLifeCost { amount: 3 })
            {
                keep = false;
            }
        }
        if keep {
            to_hand.push(card);
        } else {
            rest.push(card);
        }
    }
    put_cards(game, d, you, &to_hand, PutCards::Hand);
    put_cards(game, d, you, &rest, PutCards::Exiled);
    Some(())
}

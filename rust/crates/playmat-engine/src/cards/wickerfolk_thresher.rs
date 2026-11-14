//! Wickerfolk Thresher: "look at the top card of your library. If it's a land card, you may put it onto the
//! battlefield. If you don't put the card onto the battlefield, put it into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WickerfolkThresher.WickerfolkThresherEffect", apply)];

/// The look, a yes or no for a land, and the hand otherwise
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let &card = game.player(you).library.first()?;
    let zcc = game.obj(cx.source_id()).zcc;
    let look = crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: vec![card] };
    game.looked_at.push(look);
    if game.obj(card).def.is(CardType::Land) {
        let q = Query::YesNo { message: "Put it onto the battlefield?".into() };
        let put = loop {
            match ask(game, d, you, &q) {
                Answer::Bool(b) => break b,
                Answer::Stop => return None,
                _ => {}
            }
        };
        if put {
            enter_battlefield(game, d, card, you, false);
        }
    }
    if game.obj(card).zone != Zone::Battlefield {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}

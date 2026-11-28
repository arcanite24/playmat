//! Ms. Marvel, Kamala Khan: "Embiggen Fist — Whenever you cast a spell that targets a creature you control, draw a
//! card. Until end of turn, Ms. Marvel gains "Ms. Marvel's base power is equal to the number of cards in your hand.""

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("MsMarvelKamalaKhan.MsMarvelKamalaKhanEffect", own, Layer::SetPower(cards_in_hand))];

fn own(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// `CardsInControllerHandCount.ANY`, set as the base power alone (`SetPT_7b`)
fn cards_in_hand(game: &Game, cx: &StaticCx, _: ObjId) -> i32 {
    game.player(cx.controller).hand.len() as i32
}

//! Consuming Aberration: "Consuming Aberration's power and toughness are each equal to the number of cards in your
//! opponents' graveyards. Whenever you cast a spell, each opponent reveals cards from the top of their library until
//! they reveal a land card, then puts those cards into their graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("ConsumingAberration.CardsInOpponentsGraveyardsCount", opponents_graveyards),
    Handler::effect("ConsumingAberration.ConsumingAberrationEffect", apply),
];

/// The cards in your opponents' graveyards
fn opponents_graveyards(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    (0..game.seats()).filter(|p| *p != you).map(|p| game.player(p).graveyard.len() as i32).sum()
}

/// Each opponent in turn order reveals down to their first land (all of the library without one) and puts those
/// cards into their graveyard, in the order they were revealed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        let mut cards = Vec::new();
        for c in game.player(p).library.clone() {
            cards.push(c);
            if game.has_type(c, CardType::Land) {
                break;
            }
        }
        if cards.is_empty() {
            continue;
        }
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = cards.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
        for c in cards {
            game.move_to(c, Zone::Graveyard);
        }
    }
    Some(())
}

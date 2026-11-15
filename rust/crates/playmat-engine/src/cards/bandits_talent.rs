//! Bandit's Talent: "When Bandit's Talent enters, each opponent discards two cards unless they discard a nonland
//! card. {B}: Level 2 — At the beginning of each opponent's upkeep, if that player has one or fewer cards in hand,
//! they lose 2 life. {3}{B}: Level 3 — At the beginning of your draw step, draw an additional card for each opponent
//! who has one or fewer cards in hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BanditsTalent.BanditsTalentDiscardEffect", apply),
    Handler::value("BanditsTalent.BanditsTalentValue", opponents_with_one_or_fewer),
];

/// Each opponent with a hand, in turn: with a nonland card, "Discard a nonland card?" (one nonland card), else two
/// cards; then everyone's chosen cards are discarded
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.seats();
    let mut chosen: Vec<(Seat, Vec<ObjId>)> = Vec::new();
    for p in (0..n).map(|k| (you + k) % n) {
        if p == you || game.player(p).lost || game.player(p).hand.is_empty() {
            continue;
        }
        let hand = game.player(p).hand.clone();
        let nonland: Vec<ObjId> = hand.iter().copied().filter(|c| !game.has_type(*c, CardType::Land)).collect();
        let cards = if !nonland.is_empty() && choose_use(game, d, p, "Discard a nonland card?")? {
            choose_cards(game, d, p, &hand, &nonland, 1, 1, "discard")?
        } else {
            let k = hand.len().min(2);
            choose_cards(game, d, p, &hand, &hand, k, k, "discard")?
        };
        chosen.push((p, cards));
    }
    for (p, cards) in chosen {
        for c in cards {
            if game.obj(c).zone == Zone::Hand {
                discard_card(game, p, c);
            }
        }
    }
    game.discarded_cards();
    Some(())
}

/// the opponents with one or fewer cards in hand
fn opponents_with_one_or_fewer(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost && game.player(*p).hand.len() <= 1).count() as i32
}

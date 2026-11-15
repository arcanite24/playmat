//! Deliver Unto Evil: "Choose up to four target cards in your graveyard. If you control a Bolas planeswalker, return
//! those cards to your hand. Otherwise, an opponent chooses two of them. Leave the chosen cards in your graveyard and
//! put the rest into your hand. Exile Deliver Unto Evil."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeliverUntoEvil.DeliverUntoEvilEffect", apply)
    .filters(&["new FilterControlledPlaneswalkerPermanent(SubType.BOLAS)"])];

/// the targets to your hand with a Bolas planeswalker; else an opponent you choose (`TargetOpponent`, not targeted) chooses
/// `min(2, cards)` of them (`TargetCardInGraveyard`), and the rest go to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = cx.objs(0);
    if cards.is_empty() {
        return Some(());
    }
    if !matching(game, &cx.filters[0], you, cx.source_id()).is_empty() {
        if !put_cards(game, d, you, &cards, crate::card::PutCards::Hand) {
            return None;
        }
        return Some(());
    }
    // `player.choose` of a not-targeted `TargetOpponent`: asked even with one opponent
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    if opponents.is_empty() {
        return Some(());
    }
    let opponent = loop {
        match ask(game, d, you, &Query::Target { players: opponents.clone(), objs: Vec::new(), required: true }) {
            Answer::Player(p) if opponents.contains(&p) => break p,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let n = cards.len().min(2);
    let chosen = choose_cards_rounds(game, d, opponent, &cards, n, n)?;
    cards.retain(|c| !chosen.contains(c));
    if !put_cards(game, d, you, &cards, crate::card::PutCards::Hand) {
        return None;
    }
    Some(())
}

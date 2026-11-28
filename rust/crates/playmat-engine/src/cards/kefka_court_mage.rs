//! Kefka, Court Mage: "Whenever Kefka enters or attacks, each player discards a card. Then you draw a card for each
//! card type among cards discarded this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KefkaCourtMage.KefkaCourtMageEffect", apply)];

/// each player from you in turn order picks (`TargetDiscard`; a lone card is taken), then they discard together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut picks: Vec<(Seat, ObjId)> = Vec::new();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        let hand = game.player(p).hand.clone();
        match hand.len() {
            0 => {}
            1 => picks.push((p, hand[0])),
            _ => {
                if let Some(&c) = choose_cards(game, d, p, &hand, &hand, 1, 1, &rule)?.first() {
                    picks.push((p, c));
                }
            }
        }
    }
    let mut discarded = Vec::new();
    for (p, c) in picks {
        if game.obj(c).zone == Zone::Hand {
            discard_card(game, p, c);
            discarded.push(c);
        }
    }
    if discarded.is_empty() {
        return Some(());
    }
    let mut types: Vec<CardType> = Vec::new();
    for c in &discarded {
        for t in &game.obj(*c).def.types {
            if !types.contains(t) {
                types.push(*t);
            }
        }
    }
    draw(game, you, types.len());
    Some(())
}

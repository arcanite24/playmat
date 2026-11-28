//! Kroxa, Titan of Death's Hunger: "When Kroxa enters, sacrifice it unless it escaped. Whenever Kroxa enters or
//! attacks, each opponent discards a card, then each opponent who didn't discard a nonland card this way loses 3
//! life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KroxaTitanOfDeathsHunger.KroxaTitanOfDeathsHungerEntersEffect", enters),
    Handler::effect("KroxaTitanOfDeathsHunger.KroxaTitanOfDeathsHungerDiscardEffect", discards),
];

/// `getSourcePermanentIfItStillExists`, then `EscapeAbility.wasCastedWithEscape`
fn enters(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) && !game.obj(cx.source_id()).escaped {
        sacrifice(game, cx.source_id());
    }
    Some(())
}

/// Each opponent with cards in hand chooses one (`choose(Discard, hand, TargetCardInHand(1))`), in turn order;
/// then each chosen card is discarded in that order; then each opponent who didn't discard a nonland card loses 3
fn discards(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> =
        (1..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    let rule = item_rule(game, cx.item);
    let mut chosen = Vec::new();
    for &p in &opponents {
        let hand = game.player(p).hand.clone();
        if hand.is_empty() {
            continue;
        }
        chosen.extend(choose_cards(game, d, p, &hand, &hand, 1, 1, &rule)?);
    }
    let mut discarded_nonland = Vec::new();
    for c in chosen {
        let owner = game.obj(c).owner;
        if game.obj(c).zone != Zone::Hand {
            continue;
        }
        discard_card(game, owner, c);
        game.discarded_cards();
        if !game.has_type(c, CardType::Land) {
            discarded_nonland.push(owner);
        }
    }
    for p in opponents {
        if !discarded_nonland.contains(&p) {
            lose_life(game, p, 3);
        }
    }
    Some(())
}

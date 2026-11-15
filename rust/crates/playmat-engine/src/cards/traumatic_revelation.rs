//! Traumatic Revelation: "Target opponent reveals their hand. You may choose a creature or battle card from it. If
//! you do, that player discards that card. If you don't, incubate 3."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TraumaticRevelation.TraumaticRevelationEffect", apply).effects(&["new IncubateEffect(3)"])];

/// As Java plays it: the choice is a `TargetCardInHand`, which only takes cards in the chooser's own hand, so none of
/// the opponent's cards can be chosen. You're asked (a nonempty hand), nothing is discarded, and you incubate 3.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let hand = game.player(p).hand.clone();
    // `revealCards(source, hand, game)`: nothing shown for an empty hand
    if !hand.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        // `choose(Outcome.Discard, hand, TargetCardInHand(0, 1, filter))`: an empty hand isn't asked about
        choose_cards(game, d, you, &hand, &[], 0, 1, &item_rule(game, cx.item))?;
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}

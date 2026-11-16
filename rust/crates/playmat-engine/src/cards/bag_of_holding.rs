//! Bag of Holding: "Whenever you discard a card, exile that card from your graveyard. {2}, {T}: Draw a card, then
//! discard a card. {4}, {T}, Sacrifice Bag of Holding: Return all cards exiled with Bag of Holding to their owner's
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BagOfHolding.BagOfHoldingExileEffect", exile),
    Handler::effect("BagOfHolding.BagOfHoldingReturnCardsEffect", return_cards),
];

/// The discarded card, if you own it and it's still in a graveyard: exiled with this
fn exile(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.item.discarded?;
    let o = game.obj(card);
    if o.owner != cx.controller() || o.zone != Zone::Graveyard {
        return Some(());
    }
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        game.obj_mut(card).exiled_with = Some(cx.source);
    }
    Some(())
}

/// the cards in its exile zone, as it was when the ability was activated, to their owners' hands
fn return_cards(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &[EffectDef::ReturnFromExileForSourceEffect { zone: "hand".into() }]);
    Some(())
}

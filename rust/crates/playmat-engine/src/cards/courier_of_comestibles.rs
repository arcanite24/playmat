//! Courier of Comestibles: "When this creature enters, you may search your library for a Food card, reveal it, put
//! it into your hand, then shuffle. If you don't put a card into your hand this way, create a Food token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CourierOfComestibles.CourierOfComestiblesEffect", apply)
    .tokens(&["FoodToken"])
    .filters(&["new FilterCard(SubType.FOOD)"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Search your library for a Food card?")? {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        return Some(());
    }
    let found = search_library(game, d, you, &cx.filters[0], 0, 1, cx.source_id())?;
    let mut in_hand = false;
    if let Some(&card) = found.first() {
        let zcc = cx.source.1;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        game.move_to(card, Zone::Hand);
        in_hand = game.obj(card).zone == Zone::Hand;
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    if !in_hand {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}

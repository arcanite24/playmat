//! Embermouth Sentinel: "When Embermouth Sentinel enters, you may search your library for a basic land card, reveal
//! it, then shuffle and put that card on top. If you control a Dragon, put that card onto the battlefield tapped
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EmbermouthSentinel.EmbermouthSentinelEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// the search, the shuffle, then the card revealed and put on top (or onto the battlefield tapped with a Dragon)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 1, 1, cx.source_id())?;
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    let Some(&card) = found.first() else { return Some(()) };
    let zcc = cx.source.1;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    let dragon = game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Dragon"));
    if dragon {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    } else {
        to_library(game, card, true);
    }
    Some(())
}

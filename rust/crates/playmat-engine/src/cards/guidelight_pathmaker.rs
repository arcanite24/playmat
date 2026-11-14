//! Guidelight Pathmaker: "When this creature enters, you may search your library for an artifact card and reveal it.
//! Put it onto the battlefield if its mana value is 2 or less. Otherwise, put it into your hand. If you search your
//! library this way, shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GuidelightPathmaker.GuidelightPathmakerEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_ARTIFACT"])];

/// `TargetCardInLibrary(filter)`: the card found is revealed, then onto the battlefield (mana value 2 or less) or
/// into your hand; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc: cx.source.1,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        if game.obj(card).def.mana_value() <= 2 {
            put_onto_battlefield(game, d, card, you);
            if d.stopped() {
                return None;
            }
        } else {
            game.move_to(card, Zone::Hand);
        }
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}

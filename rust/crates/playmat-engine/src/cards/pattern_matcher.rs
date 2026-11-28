//! Pattern Matcher: "When Pattern Matcher enters the battlefield, you may search your library for a card with the
//! same name as another creature you control, reveal it, put it into your hand, then shuffle."

use super::*;
use crate::card::Pred;

pub const HANDLERS: &[Handler] = &[Handler::effect("PatternMatcher.RegularExpression", apply)];

/// the names of your other creatures (`FILTER_CONTROLLED_ANOTHER_CREATURE`, none empty) as an or of `NamePredicate`s
/// (an empty or matches nothing), then `SearchLibraryPutInHandEffect(TargetCardInLibrary(filter), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let names: Vec<Pred> = controlled_creatures(game, you)
        .into_iter()
        .filter(|c| *c != cx.source_id())
        .map(|c| game.obj(c).name().to_string())
        .filter(|n| !n.is_empty())
        .map(|n| Pred::Name { n })
        .collect();
    let filter = Filter(vec![Pred::Or { xs: names }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc: cx.source.1,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        game.move_to(card, Zone::Hand);
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}

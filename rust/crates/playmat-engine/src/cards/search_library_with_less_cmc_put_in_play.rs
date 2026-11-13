//! `SearchLibraryWithLessCMCPutInPlayEffect` (Whir of Invention): "Search your library for an artifact card with mana
//! value X or less, put it onto the battlefield, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SearchLibraryWithLessCMCPutInPlayEffect", apply)];

/// the filter argument with `ManaValuePredicate(FEWER_THAN, X + 1)` added, then `SearchLibraryEffect`'s search and
/// shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut filter = match cx.args.first().map(|s| s.trim()) {
        Some("StaticFilters.FILTER_CARD_ARTIFACT") | Some("StaticFilters.FILTER_CARD_ARTIFACT_AN") => {
            crate::card::Filter::of(vec![Pred::Type { t: CardType::Artifact }])
        }
        // Chord of Calling
        Some("StaticFilters.FILTER_CARD_CREATURE") => crate::card::Filter::of(vec![Pred::Type { t: CardType::Creature }]),
        _ => return Some(()),
    };
    filter.0.push(Pred::Mv { cmp: crate::card::Cmp::FewerThan, n: cx.item.x.max(0) + 1 });
    let e = EffectDef::SearchLibraryEffect {
        filter,
        min: 1,
        max: 1,
        to: crate::card::PutCards::Battlefield,
        optional: false,
        target_controller: false,
    };
    cx.run(game, d, &[e]);
    Some(())
}

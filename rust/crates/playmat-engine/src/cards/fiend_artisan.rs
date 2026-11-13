//! Fiend Artisan's "{X}{B/G}, {T}, Sacrifice another creature: Search your library for a creature card with mana
//! value X or less, put it onto the battlefield, then shuffle. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FiendArtisan.FiendArtisanEffect", apply)];

/// `SearchLibraryPutInPlayEffect(TargetCardInLibrary(filter))`, the filter a creature card with mana value less
/// than X + 1 for the ability's X
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let filter = Filter(vec![Pred::Type { t: CardType::Creature }, Pred::Mv { cmp: Cmp::FewerThan, n: cx.item.x + 1 }]);
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

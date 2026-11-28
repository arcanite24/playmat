//! The Earth King: "Whenever one or more creatures you control with power 4 or greater attack, search your library
//! for up to that many basic land cards, put them onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheEarthKing.TheEarthKingEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LANDS", "filter"])];

/// "That many": the attacking creatures of the trigger's filter (`VALUEKEY_NUMBER_ATTACKERS`), counted as it
/// resolves; then `SearchLibraryPutInPlayEffect(TargetCardInLibrary(0, that many, basic lands), tapped)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let [lands, attackers] = cx.filters else { return Some(()) };
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).attacking.is_some() && game.filter_matches(attackers, you, Some(cx.source_id()), **o))
        .count();
    let e = EffectDef::SearchLibraryEffect {
        filter: lands.clone(),
        min: 0,
        max: n,
        to: crate::card::PutCards::BattlefieldTapped,
        optional: false,
        target_controller: false,
    };
    cx.run(game, d, &[e]);
    Some(())
}

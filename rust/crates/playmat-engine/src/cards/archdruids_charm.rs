//! Archdruid's Charm's first mode: "Search your library for a creature or land card and reveal it. Put it onto the
//! battlefield tapped if it's a land card. Otherwise, put it into your hand. Then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArchdruidsCharm.ArchdruidsCharmMode1Effect", apply)
    .args(&["library_target", "text", "text"])];

/// The search (`TargetCardInLibrary(creatureOrLandFilter)`), each card found moved (a land onto the battlefield
/// tapped, else to the hand), then revealed where it went; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    let mut shown = Vec::new();
    for card in found {
        if game.has_type(card, CardType::Land) {
            if !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped) {
                return None;
            }
        } else {
            game.move_to(card, Zone::Hand);
        }
        shown.push((card, game.obj(card).zone));
    }
    if !shown.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

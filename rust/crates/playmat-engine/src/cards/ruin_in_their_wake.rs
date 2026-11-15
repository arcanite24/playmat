//! Ruin in Their Wake: "Devoid. Search your library for a basic land card and reveal it. You may put that card onto
//! the battlefield tapped if you control a land named Wastes. Otherwise, put that card into your hand. Then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RuinInTheirWake.RuinInTheirWakeEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// `TargetCardInLibrary(basic land)`: the card found is revealed; with a Wastes among your lands "Put <it> onto
/// battlefield tapped?", else (or on a no) into your hand; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = vec![(card, Zone::Library)];
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        let wastes = game.battlefield.iter().any(|l| {
            game.obj(*l).controller == you && game.has_type(*l, CardType::Land) && game.obj(*l).def.name == "Wastes"
        });
        let message = format!("Put {} onto battlefield tapped?", log::object(game, card));
        let how = if wastes && choose_use(game, d, you, &message)? {
            crate::card::PutCards::BattlefieldTapped
        } else {
            crate::card::PutCards::Hand
        };
        if !put_cards(game, d, you, &[card], how) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

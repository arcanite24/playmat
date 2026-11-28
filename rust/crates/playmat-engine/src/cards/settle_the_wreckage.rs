//! Settle the Wreckage: "Exile all attacking creatures target player controls. That player may search their library
//! for that many basic land cards, put those cards onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SettleTheWreckage.SettleTheWreckageEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// The target player's attacking creatures (none: done) are exiled; then that player is asked "Search your library
/// for up to N basic land card(s)?" and, yes, searches for up to that many (`TargetCardInLibrary(0, N, …)`), put
/// onto the battlefield tapped under their control. Java's `apply` doesn't shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let attackers: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == p && game.obj(*o).attacking.is_some() && game.has_type(*o, CardType::Creature)
        })
        .collect();
    let n = attackers.len();
    if n == 0 {
        return Some(());
    }
    for o in &attackers {
        game.move_to(*o, Zone::Exile);
    }
    let what = if n > 1 { "basic land cards" } else { "basic land card" };
    // (`CardUtil.numberToText(n)`: "one" for 1)
    let count = if n == 1 { "one".to_string() } else { log::number(n) };
    let message = format!("Search your library for up to {count} {what}?");
    if !choose_use(game, d, p, &message)? {
        return Some(());
    }
    let filter = cx.filters[0].clone();
    let found = search_library(game, d, p, &filter, 0, n, cx.source_id())?;
    if !found.is_empty() && !put_cards(game, d, p, &found, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}

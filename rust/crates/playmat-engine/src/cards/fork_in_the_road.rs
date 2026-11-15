//! Fork in the Road: "Search your library for up to two basic land cards and reveal them. Put one into your hand and
//! the other into your graveyard. Then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ForkInTheRoad.ForkInTheRoadEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// Up to two found and revealed; you choose the one for your hand, the other goes to the graveyard; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut found = search_library(game, d, you, &cx.filters[0], 0, 2, cx.source_id())?;
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        let rule = item_rule(game, cx.item);
        if let Some(&c) = choose_cards(game, d, you, &found, &found, 1, 1, &rule)?.first() {
            game.move_to(c, Zone::Hand);
            found.retain(|x| *x != c);
        }
        if let Some(&c) = found.first() {
            game.move_to(c, Zone::Graveyard);
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

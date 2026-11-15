//! Liliana, Death Mage: "+1: Return up to one target creature card from your graveyard to your hand. −3: Destroy
//! target creature. Its controller loses 2 life. −7: Target opponent loses 2 life for each creature card in their
//! graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LilianaDeathMage.LilianaDeathMagePlusEffect", plus),
    Handler::effect("LilianaDeathMage.LilianaDeathMageUltimateEffect", ultimate)
        .filters(&["StaticFilters.FILTER_CARD_CREATURE"]),
];

/// the target card (if one was chosen) to your hand
fn plus(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    if !put_cards(game, d, cx.controller(), &[card], crate::card::PutCards::Hand) {
        return None;
    }
    Some(())
}

/// the target opponent loses twice the creature cards in their graveyard
fn ultimate(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let opponent = cx.player(0)?;
    let src = cx.source_id();
    let you = cx.controller();
    let n = game
        .player(opponent)
        .graveyard
        .iter()
        .filter(|c| game.filter_matches(&cx.filters[0], you, Some(src), **c))
        .count() as i32;
    lose_life(game, opponent, n * 2);
    Some(())
}

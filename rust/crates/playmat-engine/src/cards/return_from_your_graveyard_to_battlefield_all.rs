//! `ReturnFromYourGraveyardToBattlefieldAllEffect(filter, tapped)` (World Shaper…): "return all land cards from your
//! graveyard to the battlefield tapped." (The one-argument form is the engine's own effect.)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ReturnFromYourGraveyardToBattlefieldAllEffect", apply).args(&["filter", "text"])];

/// `moveCards` of them all at once, tapped by the second argument
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let to = if cx.args.get(1).map(|a| a.trim()) == Some("true") {
        crate::card::PutCards::BattlefieldTapped
    } else {
        crate::card::PutCards::Battlefield
    };
    let cards: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.filter_matches(&filter, you, Some(cx.source_id()), *c))
        .collect();
    if !cards.is_empty() && !put_cards(game, d, you, &cards, to) {
        return None;
    }
    Some(())
}

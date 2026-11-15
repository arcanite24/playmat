//! Experimental Overload: "Create an X/X blue and red Weird creature token, where X is the number of instant and
//! sorcery cards in your graveyard. Then you may return an instant or sorcery card from your graveyard to your hand.
//! Exile Experimental Overload."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExperimentalOverload.ExperimentalOverloadEffect", apply)
    .tokens(&["WeirdXXToken"])
    .filters(&["StaticFilters.FILTER_CARD_INSTANT_OR_SORCERY"])];

/// The token for the instant and sorcery cards counted, then (with any) up to one of them chosen
/// (`TargetCardInYourGraveyard(0, 1, …)` with `Player.choose`) and returned to the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spells = |game: &Game| -> Vec<ObjId> {
        let grave = game.player(you).graveyard.clone();
        grave.into_iter().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect()
    };
    let n = spells(game).len() as i32;
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(n) };
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    if n < 1 {
        return Some(());
    }
    let grave = game.player(you).graveyard.clone();
    let fitting = spells(game);
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &grave, &fitting, 0, 1, &rule)?;
    if let Some(&c) = chosen.first() {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}

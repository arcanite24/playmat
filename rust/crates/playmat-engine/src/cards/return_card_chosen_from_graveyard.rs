//! `ReturnCardChosenFromGraveyardEffect(optional, filter, putCards)` (Blossoming Tortoise): "[you may] return a land
//! card from your graveyard to the battlefield tapped": a card of the filter chosen from your graveyard (not
//! targeted), put into the hand or onto the battlefield (tapped).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ReturnCardChosenFromGraveyardEffect", apply)
    .args(&["one_of:true|false", "filter", "one_of:PutCards.HAND|PutCards.BATTLEFIELD|PutCards.BATTLEFIELD_TAPPED"])];

/// None of the filter in your graveyard: nothing. Else `choose(TargetCardInYourGraveyard(optional ? 0 : 1, 1, filter,
/// true))`, then `putCards.moveCards` of what was chosen
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(filter), [optional, _, to]) = (cx.filters.first(), cx.args) else { return Some(()) };
    let to = to.trim().strip_prefix("PutCards.")?;
    let to = serde_json::from_value::<crate::card::PutCards>(serde_json::Value::String(to.into())).ok()?;
    let you = cx.controller();
    let graveyard = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> =
        graveyard.iter().copied().filter(|c| game.filter_matches(filter, you, Some(cx.source_id()), *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let min = if optional.trim() == "true" { 0 } else { 1 };
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &graveyard, &possible, min, 1, &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, to) {
        return None;
    }
    Some(())
}

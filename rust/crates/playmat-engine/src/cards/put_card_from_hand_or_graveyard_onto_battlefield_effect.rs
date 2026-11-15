//! `PutCardFromHandOrGraveyardOntoBattlefieldEffect(filter, tapped)` (Bonny Pall, Clearcutter): "you may put a land
//! card from your hand or graveyard onto the battlefield [tapped]"

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("PutCardFromHandOrGraveyardOntoBattlefieldEffect", apply).args(&["filter", "one_of:true|false"])];

/// The filter's cards in your hand, then your graveyard; none: nothing. Else `choose(TargetCard(0, 1, ALL))` of
/// them, the one chosen put onto the battlefield (tapped)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(filter), [_, tapped]) = (cx.filters.first(), cx.args) else { return Some(()) };
    let you = cx.controller();
    let src = cx.source_id();
    let player = game.player(you);
    let cards: Vec<ObjId> = player
        .hand
        .iter()
        .chain(player.graveyard.iter())
        .copied()
        .filter(|c| game.filter_matches(filter, you, Some(src), *c))
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &cards, &cards, 0, 1, &rule)?;
    let to = if tapped.trim() == "true" {
        crate::card::PutCards::BattlefieldTapped
    } else {
        crate::card::PutCards::Battlefield
    };
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, to) {
        return None;
    }
    Some(())
}

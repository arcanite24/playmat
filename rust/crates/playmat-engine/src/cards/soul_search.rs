//! Soul Search: "Target opponent reveals their hand. You choose a nonland card from it. Exile that card. If the
//! card's mana value is 1 or less, create a 1/1 white and black Spirit creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SoulSearch.SoulSearchEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_NON_LAND"])
    .tokens(&["WhiteBlackSpiritToken"])];

/// The reveal (not of an empty hand); a nonland card of it (`TargetCard(1, HAND)`, chosen by you) exiled, and the
/// Spirit for mana value 1 or less
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return None;
    }
    let zcc = game.obj(cx.source_id()).zcc;
    let cards = hand.iter().map(|c| (*c, Zone::Hand)).collect();
    game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    if possible.is_empty() {
        return Some(());
    }
    // `SpellAbility.getRule(true)` ends with the ability's name, "Cast Soul Search": its "search" keeps a lone
    // nonland card from being picked by itself
    let rule = format!("{}Cast {}", item_rule(game, cx.item), game.obj(cx.source_id()).def.name);
    let &card = choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?.first()?;
    game.move_to(card, Zone::Exile);
    if game.obj(card).def.mana_value() <= 1 {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
    }
    Some(())
}

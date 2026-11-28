//! Deep-Cavern Bat: "When this creature enters, look at target opponent's hand. You may exile a nonland card from it
//! until this creature leaves the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeepCavernBat.DeepCaverBatEffect", apply)];

/// Nothing with an empty hand; you look at it even if the Bat is gone, then (the Bat still there)
/// `choose(hand, TargetCard(0, 1, nonland))`, asked even with no nonland card; the card chosen is exiled until the
/// Bat leaves and then returns to its owner's hand (`OnLeaveReturnExiledAbility(Zone.HAND)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let (src, zcc) = cx.source;
    let look = crate::state::LookedAt { viewer: you, source: src, zcc: game.obj(src).zcc, bare: false, cards: hand.clone() };
    game.looked_at.push(look);
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let possible: Vec<ObjId> = hand.iter().copied().filter(|c| !game.has_type(*c, CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &possible, 0, 1, &rule)?;
    for c in chosen {
        game.move_to(c, Zone::Exile);
        game.exiled_until_leaves.push((src, zcc, c));
        game.return_to_hand.push(c);
    }
    Some(())
}

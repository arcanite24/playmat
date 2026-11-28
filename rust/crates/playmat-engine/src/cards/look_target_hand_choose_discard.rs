//! `LookTargetHandChooseDiscardEffect(upTo, amount[, filter])` (Cracked Skull): "look at target player's hand. You
//! may choose a nonland card from it. That player discards that card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("LookTargetHandChooseDiscardEffect", apply).args(&["bool", "text", "filter"])];

/// `controller.choose(hand, TargetCard(upTo ? 0 : n, n, filter))`, then the target player discards what was chosen;
/// with none to choose (n = 0), only a look at the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let up_to = cx.args.first().is_some_and(|a| a.trim() == "true");
    let n = cx.args.get(1).map(|a| a.trim().trim_start_matches("StaticValue.get(").trim_end_matches(')'))?;
    let n: usize = n.parse().ok()?;
    let hand = game.player(p).hand.clone();
    if n == 0 {
        if !hand.is_empty() {
            let (source, zcc) = cx.source;
            game.looked_at.push(crate::state::LookedAt { viewer: you, source, zcc, bare: false, cards: hand });
        }
        return Some(());
    }
    let possible: Vec<ObjId> = match cx.filters.first() {
        Some(f) => hand.iter().copied().filter(|c| game.filter_matches(f, you, Some(cx.source_id()), *c)).collect(),
        None => hand.clone(),
    };
    let rule = item_rule(game, cx.item);
    let min = if up_to { 0 } else { n.min(possible.len()) };
    let chosen = choose_cards(game, d, you, &hand, &possible, min, n, &rule)?;
    let chosen: Vec<ObjId> = chosen.into_iter().filter(|c| game.obj(*c).zone == Zone::Hand).collect();
    if chosen.is_empty() {
        return Some(());
    }
    for c in chosen {
        discard_card(game, p, c);
    }
    game.discarded_cards();
    Some(())
}

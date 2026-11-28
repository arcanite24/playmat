//! Mind Roots: "Target player discards two cards. Put up to one land card discarded this way onto the battlefield
//! tapped under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MindRoots.MindRootsEffect", apply)];

/// `discard(2, 2)`, then `choose(cards, TargetCard(0, 1, land))` among the discarded cards
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let you = cx.controller();
    let cards = discard(game, d, p, 2);
    if d.stopped() {
        return None;
    }
    let lands: Vec<ObjId> =
        cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Graveyard && game.obj(*c).is(CardType::Land)).collect();
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    if let Some(&l) = choose_cards(game, d, you, &cards, &lands, 0, 1, &rule)?.first() {
        if !put_cards(game, d, you, &[l], crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    }
    Some(())
}

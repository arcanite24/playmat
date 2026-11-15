//! See the Truth: "Look at the top three cards of your library. Put one of those cards into your hand and the rest
//! on the bottom of your library in any order. If this spell was cast from anywhere other than your hand, put each
//! of those cards into your hand instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SeeTheTruth.SeeTheTruthEffect", apply)];

/// Cast this turn from somewhere other than the hand (`SpellsCastWatcher`): all three to the hand. Else (from the
/// hand, or a copy that wasn't cast) "card to put into your hand", the rest to the bottom in any order.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let cast = game.watch.spells.iter().rev().find(|s| s.card == cx.item.card);
    let from_hand = cast.is_none_or(|s| s.from == Some(Zone::Hand));
    if !from_hand {
        for c in cards {
            game.move_to(c, Zone::Hand);
        }
        return Some(());
    }
    let chosen = choose_cards(game, d, you, &cards, &cards, 1, 1, "card to put into your hand")?;
    for c in &chosen {
        game.move_to(*c, Zone::Hand);
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| !chosen.contains(c)).collect();
    put_on_library(game, d, you, &rest, false, true).then_some(())
}

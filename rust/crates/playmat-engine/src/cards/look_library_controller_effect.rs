//! `LookLibraryControllerEffect(n)` (Ponder): "Look at the top N cards of your library, then put them back in any
//! order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LookLibraryControllerEffect", apply).args(&["amount"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.arg_amount(game, 0).unwrap_or(1).max(0) as usize;
    let cards: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &cards, crate::card::PutCards::TopAny) {
        return None;
    }
    Some(())
}

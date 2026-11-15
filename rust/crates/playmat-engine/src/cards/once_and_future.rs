//! Once and Future: "Return target card from your graveyard to your hand. Put up to one other target card from your
//! graveyard on top of your library. Exile Once and Future. Adamant — If at least three green mana was spent to cast
//! this spell, instead return those cards to your hand and exile Once and Future."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("OnceAndFuture.OnceAndFutureEffect", apply).effects(&["new ExileSpellEffect()"])];

/// adamant (`AdamantCondition.GREEN`): both targets to your hand together; else the first to your hand and the
/// second on top of your library; then `ExileSpellEffect`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let first = cx.obj(0);
    let second = cx.obj(1);
    if game.obj(cx.source_id()).spent_colors[4] >= 3 {
        let cards: Vec<ObjId> = first.into_iter().chain(second).collect();
        if !put_cards(game, d, you, &cards, crate::card::PutCards::Hand) {
            return None;
        }
    } else {
        if let Some(c) = first {
            if !put_cards(game, d, you, &[c], crate::card::PutCards::Hand) {
                return None;
            }
        }
        if let Some(c) = second {
            if !put_on_library(game, d, you, &[c], true, true) {
                return None;
            }
        }
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}

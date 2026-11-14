//! Dimensional Infiltrator: "{1}{C}: Exile the top card of target opponent's library. If it's a land card, you may
//! return Dimensional Infiltrator to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DimensionalInfiltrator.DimensionalInfiltratorEffect", apply)];

/// The opponent's top card exiled; a land lets you (`chooseUse`) return the Infiltrator, if it's still there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let Some(&card) = game.player(p).library.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if !game.obj(card).is(CardType::Land) || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let src = cx.source_id();
    let message = format!("Return {} to its owner's hand?", game.obj(src).name());
    if choose_use(game, d, you, &message)? {
        put_cards(game, d, you, &[src], crate::card::PutCards::Hand);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

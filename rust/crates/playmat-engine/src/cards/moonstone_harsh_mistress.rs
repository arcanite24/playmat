//! Moonstone, Harsh Mistress: "Flying. Whenever you discard a card, you may exile that card from your graveyard. If
//! you do, until the end of your next turn, you may play that card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MoonstoneHarshMistress.MoonstoneHarshMistressEffect", apply)];

/// The discarded card, if it's still in a graveyard: exiled (with this source's exile zone), then playable by you
/// until the end of your next turn (`PlayFromNotOwnHandZoneTargetEffect(Zone.EXILED, UntilEndOfYourNextTurn)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.item.discarded?;
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let you = cx.controller();
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    let o = game.obj_mut(card);
    o.exiled_with = Some(cx.source);
    o.may_play = Some((you, until));
    o.may_play_end_step = false;
    Some(())
}

//! Practiced Scrollsmith: "First strike. When this creature enters, exile target noncreature, nonland card from your
//! graveyard. Until the end of your next turn, you may cast that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PracticedScrollsmith.PracticedScrollsmithEffect", apply)];

/// `exileAndPlayFromExile(YOU, UntilEndOfYourNextTurn, withoutMana = false, anyColor = false, onlyCast = true)`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    // through the end of your next turn (`ExileTopXMayPlayUntilEffect`'s reckoning)
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    let o = game.obj_mut(card);
    o.may_play = Some((you, until));
    o.may_play_end_step = false;
    o.may_play_while = None;
    o.cast_only = true;
    Some(())
}

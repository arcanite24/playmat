//! Nahiri, Forged in Fury: "Affinity for Equipment. Whenever an equipped creature you control attacks, exile the top
//! card of your library. You may play that card this turn. You may cast Equipment spells this way without paying
//! their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NahiriForgedInFury.NahiriForgedInFuryEffect", apply)];

/// Your top card exiled (`exileAndPlayFromExile`): you may play it this turn, an Equipment (as it was in the library)
/// without paying its mana cost
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let &card = game.player(you).library.first()?;
    let without_mana = game.obj(card).def.has_subtype("Equipment");
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let turn = game.turn;
        let o = game.obj_mut(card);
        o.may_play = Some((you, turn));
        o.may_play_end_step = false;
        o.may_play_while = None;
        o.play_free = without_mana;
    }
    Some(())
}

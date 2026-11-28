//! Narset, Enlightened Master: "Whenever Narset attacks, exile the top four cards of your library. Until end of turn,
//! you may cast noncreature cards exiled with Narset this turn without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("NarsetEnlightenedMaster.NarsetEnlightenedMasterExileEffect", apply)];

/// the top four exiled; each noncreature, nonland one still in exile may be cast this turn without paying its mana
/// cost (`NarsetEnlightenedMasterCastFromExileEffect`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    for c in cards {
        let o = game.obj(c);
        if o.zone == Zone::Exile && !o.is(CardType::Creature) && !o.is(CardType::Land) {
            let turn = game.turn;
            let o = game.obj_mut(c);
            o.may_play = Some((you, turn));
            o.may_play_end_step = false;
            o.may_play_while = None;
            o.play_free = true;
        }
    }
    Some(())
}

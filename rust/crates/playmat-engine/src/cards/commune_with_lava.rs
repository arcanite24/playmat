//! Commune with Lava: "Exile the top X cards of your library. Until the end of your next turn, you may play those
//! cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CommuneWithLava.CommuneWithLavaEffect", apply)];

/// Into the spell's exile zone, playable through your next turn (this one too when cast on an opponent's turn)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.item.x.max(0) as usize;
    let cards: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
        if game.obj(*c).zone == Zone::Exile {
            game.obj_mut(*c).exiled_with = Some(cx.source);
        }
    }
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).may_play = Some((you, until));
        }
    }
    Some(())
}

//! Hex Magic: "Exile all the cards from your hand, then draw that many cards. Until the end of your next turn, you
//! may play cards exiled this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HexMagic.HexMagicEffect", apply)];

/// `makeCardPlayable(…, Duration.UntilEndOfYourNextTurn)`, as `ExileTopXMayPlayUntilEffect` keeps it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).hand.clone();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
        if game.obj(*c).zone == Zone::Exile {
            game.obj_mut(*c).exiled_with = Some(cx.source);
        }
    }
    draw(game, you, cards.len());
    let until = if game.active == Some(you) { game.turn + game.seats() as u32 } else { game.turn + 1 };
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).may_play = Some((you, until));
        }
    }
    Some(())
}

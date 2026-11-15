//! Invasion of Kaldheim: "When Invasion of Kaldheim enters, exile all cards from your hand, then draw that many
//! cards. Until the end of your next turn, you may play cards exiled this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InvasionOfKaldheim.InvasionOfKaldheimEffect", apply)];

/// Nothing with an empty hand; the hand exiled, as many cards drawn, then each card exiled playable by you
/// through your next turn (`makeCardPlayable(…, Duration.UntilEndOfYourNextTurn)`; this one too when it's an
/// opponent's)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards = game.player(you).hand.clone();
    if cards.is_empty() {
        return Some(());
    }
    for c in &cards {
        game.move_to(*c, Zone::Exile);
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

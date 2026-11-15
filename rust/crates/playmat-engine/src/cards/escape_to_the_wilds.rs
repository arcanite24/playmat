//! Escape to the Wilds: "Exile the top five cards of your library. You may play cards exiled this way until the end
//! of your next turn. You may play an additional land this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EscapeToTheWilds.EscapeToTheWildsEffect", apply)];

/// the top five exiled, each playable until `EscapeToTheWildsMayPlayEffect.isInactive`: the end step of a later
/// turn of yours (this turn's end step doesn't end it); then one more land this turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let until = game.turn + game.seats() as u32;
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            let o = game.obj_mut(c);
            o.may_play = Some((you, until));
            o.may_play_end_step = true;
            o.may_play_while = None;
        }
    }
    cx.run(game, d, &[EffectDef::PlayAdditionalLandsThisTurnEffect { amount: 1 }]);
    Some(())
}

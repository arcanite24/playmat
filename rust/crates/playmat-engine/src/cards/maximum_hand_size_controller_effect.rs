//! `MaximumHandSizeControllerEffect(…, Duration.EndOfGame, …)` as it resolves: "you have no maximum hand size for the
//! rest of the game" (`Integer.MAX_VALUE`, `SET`: Sea Gate Restoration) or "your maximum hand size is reduced by three
//! for the rest of the game" (`3`, `REDUCE`: Inspired Idea; no other card has either pair the other way round)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MaximumHandSizeControllerEffect", apply).args(&[
    "one_of:Integer.MAX_VALUE|3",
    "one_of:Duration.EndOfGame",
    "one_of:MaximumHandSizeControllerEffect.HandSizeModification.SET|HandSizeModification.SET|MaximumHandSizeControllerEffect.HandSizeModification.REDUCE",
])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.args.get(2).is_some_and(|m| m.trim().ends_with("REDUCE")) {
        let n = cx.arg_int(0).unwrap_or(0).max(0) as u32;
        let p = game.player_mut(cx.controller());
        p.max_hand_reduction = p.max_hand_reduction.saturating_add(n);
    } else {
        game.player_mut(cx.controller()).no_max_hand_size = true;
    }
    Some(())
}

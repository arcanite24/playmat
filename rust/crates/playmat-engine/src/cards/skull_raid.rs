//! Skull Raid: "Target opponent discards two cards. If fewer than two cards were discarded this way, you draw cards
//! equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkullRaid.SkullRaidEffect", apply)];

/// `opponent.discard(2, false, false, …)`, then the difference drawn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let before = game.player(p).hand.len();
    cx.run(game, d, &[EffectDef::DiscardTargetEffect { amount: Amount::Fixed(2) }]);
    if d.stopped() {
        return None;
    }
    let discarded = before.saturating_sub(game.player(p).hand.len()).min(2);
    if discarded < 2 {
        draw(game, cx.controller(), 2 - discarded);
    }
    Some(())
}

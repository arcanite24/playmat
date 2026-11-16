//! Into the Night: "It becomes night. Discard any number of cards, then draw that many cards plus one."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("IntoTheNight.IntoTheNightEffect", apply)
    .effects(&["new DiscardAndDrawThatManyEffect(Integer.MAX_VALUE, 1)"])];

/// `game.setDaytime(false)` (`BECOMES_DAY_NIGHT` only when day becomes night), then `player.discard(0, MAX)` and
/// that many plus one drawn: `DiscardAndDrawThatManyEffect`'s own apply
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.day == Some(true) {
        game.day = Some(false);
        let sources = game.trigger_sources();
        let fired = game.triggers(sources, |_, _, _, t| matches!(t, Trigger::DayNightChanges));
        game.pending.extend(fired);
    } else {
        game.day = Some(false);
    }
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    Some(())
}

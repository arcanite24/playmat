//! Sanctuary Wall: "Defender. {2}{W}, {T}: Tap target creature. You may put a stun counter on it. If you do, put a
//! stun counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SanctuaryWall.SanctuaryWallEffect", apply)];

/// The target still there: "Put a stun counter on …?", then one on it and, when that one went on, one on the source
/// if it's still on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let message = format!("Put a stun counter on {}?", log::object(game, t));
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    if add_counters_by(game, t, "stun", 1, Some(you)) <= 0 {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        add_counters_by(game, cx.source_id(), "stun", 1, Some(you));
    }
    Some(())
}

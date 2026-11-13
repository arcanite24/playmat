//! `AddCountersAttachedEffect(counter, text)` (Ordeal of Nylea): "put a +1/+1 counter on enchanted creature"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AddCountersAttachedEffect", apply).args(&["counter", "text"])];

/// The permanent the source is attached to gets the counter (nothing once the source has left the battlefield)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.args.len() != 2 || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let (name, n) = cx.args[0].rsplit_once(' ')?;
    let n: i32 = n.parse().ok()?;
    let Some(to) = game.attached(cx.source_id()) else { return Some(()) };
    if game.obj(to).zone == Zone::Battlefield {
        add_counters_by(game, to, name, n, Some(cx.controller()));
    }
    Some(())
}

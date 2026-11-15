//! Ballista Watcher: "{2}{R}, {T}: Ballista Watcher deals 1 damage to any target. Daybound." Ballista Wielder:
//! "{2}{R}: Ballista Wielder deals 1 damage to any target. A creature dealt damage this way can't block this turn.
//! Nightbound."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BallistaWatcher.BallistaWielderEffect", apply)
    .effects(&["new CantBlockTargetEffect(Duration.EndOfTurn)"])];

/// 1 damage to the target; a permanent dealt damage gets `CantBlockTargetEffect(EndOfTurn)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.target(0)?;
    let dealt = deal_damage(game, cx.source_id(), target, 1, false);
    if let Target::Obj(o) = target {
        if dealt > 0 {
            cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(o)]);
        }
    }
    Some(())
}

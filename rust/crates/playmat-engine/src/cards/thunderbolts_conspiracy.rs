//! Thunderbolts Conspiracy: "Whenever a Villain you control dies, return it to the battlefield under its owner's
//! control with a finality counter on it. That creature is a Hero in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThunderboltsConspiracy.ThunderboltsConspiracyEffect", apply)];

/// A permanent card still in its graveyard onto the battlefield under its owner's control with a finality counter
/// (`setEnterWithCounters`), then a Hero for as long as it stays (`AddCardSubTypeTargetEffect`, `Duration.Custom`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard || game.obj(c).token || !game.obj(c).def.is_permanent_card() {
        return Some(());
    }
    let owner = game.obj(c).owner;
    enter_battlefield(game, d, c, owner, false);
    if d.stopped() {
        return None;
    }
    if game.obj(c).zone == Zone::Battlefield {
        add_counters_by(game, c, "finality", 1, Some(cx.controller()));
        let zcc = game.obj(c).zcc;
        game.subtypes_while_there.push((c, zcc, "Hero".into()));
    }
    Some(())
}

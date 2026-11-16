//! Boros Battleshaper: "At the beginning of each combat, up to one target creature attacks or blocks this combat if
//! able and up to one target creature can't attack or block this combat."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BorosBattleshaper.BorosBattleshaperEffect", apply).effects(&[
    "new BlocksIfAbleTargetEffect(Duration.EndOfTurn)",
    "new AttacksIfAbleTargetEffect(Duration.EndOfTurn)",
    "new CantBlockTargetEffect(Duration.EndOfTurn)",
    "new CantAttackTargetEffect(Duration.EndOfTurn)",
])];

/// The first target blocks if able when its controller isn't the active player, else attacks if able; the second
/// can't block, or can't attack, the same way
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let active = game.active;
    for (group, first) in [(0, 0), (1, 2)] {
        let Some(c) = cx.obj(group) else { continue };
        let blocks = active.is_some_and(|a| a != game.obj(c).controller);
        let e = &cx.effects[if blocks { first } else { first + 1 }];
        cx.run_on(game, d, std::slice::from_ref(e), vec![Target::Obj(c)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

//! Self-Destruct: "Target creature you control deals X damage to any other target and X damage to itself, where X is
//! its power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SelfDestruct.SelfDestructEffect", apply)];

/// Both targets still legal (`EachTargetPointer`: fewer than two, nothing), the creature's power above 0: it deals
/// that much to the other target, then to itself
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    let other = cx.target(1)?;
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    let power = game.power_toughness(creature).0;
    if power < 1 {
        return Some(());
    }
    let other_there = match other {
        Target::Obj(o) => game.obj(o).zone == Zone::Battlefield,
        Target::Player(_) => true,
    };
    if other_there {
        deal_damage(game, creature, other, power, false);
    }
    if game.obj(creature).zone == Zone::Battlefield {
        deal_damage(game, creature, Target::Obj(creature), power, false);
    }
    Some(())
}

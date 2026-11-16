//! Monstrous Step: "Target creature gets +7/+7 until end of turn. Up to one other target creature blocks it this
//! turn if able. Cycling {2}"

use super::*;
use crate::state::{Requirement, RequirementKind};

pub const HANDLERS: &[Handler] = &[Handler::effect("MonstrousStep.MonstrousStepEffect", apply)];

/// `MonstrousStepEffect` (a `RequirementEffect` until end of turn): the second target must block the first this
/// turn if able, while both are still those permanents
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(attacker), Some(blocker)) = (cx.obj(0), cx.obj(1)) else { return Some(()) };
    if game.obj(attacker).zone != Zone::Battlefield || game.obj(blocker).zone != Zone::Battlefield {
        return Some(());
    }
    let req = Requirement {
        kind: RequirementKind::MustBlock,
        obj: (blocker, game.obj(blocker).zcc),
        attacker: Some((attacker, game.obj(attacker).zcc)),
        end_of_combat: false,
    };
    game.requirements.push(req);
    Some(())
}

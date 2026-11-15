//! Fighter Class, level 3: "Whenever a creature you control attacks, up to one target creature blocks it this combat
//! if able."

use super::*;
use crate::state::{Requirement, RequirementKind};

pub const HANDLERS: &[Handler] = &[Handler::effect("FighterClass.FighterClassEffect", apply)];

/// `FighterClassEffect`: a `FighterClassRequirementEffect` until end of combat, the target must block the attacker
/// (`attackerRef`, the trigger's attacker) while both are still those permanents
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(attacker), Some(blocker)) = (cx.item.entering, cx.obj(0)) else { return Some(()) };
    if game.obj(attacker).zone != Zone::Battlefield || game.obj(blocker).zone != Zone::Battlefield {
        return Some(());
    }
    let req = Requirement {
        kind: RequirementKind::MustBlock,
        obj: (blocker, game.obj(blocker).zcc),
        attacker: Some((attacker, game.obj(attacker).zcc)),
        end_of_combat: true,
    };
    game.requirements.push(req);
    Some(())
}

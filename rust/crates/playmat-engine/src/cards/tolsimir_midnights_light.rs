//! Tolsimir, Midnight's Light: "Whenever a Wolf you control attacks, if Tolsimir attacked this combat, target
//! creature an opponent controls blocks that Wolf this combat if able."

use super::*;
use crate::state::{Requirement, RequirementKind};

pub const HANDLERS: &[Handler] = &[
    Handler::condition("TolsimirMidnightsLight.TolsimirMidnightsLightCondition", attacked),
    Handler::effect("TolsimirMidnightsLight.TolsimirMidnightsLightEffect", apply),
];

/// Tolsimir (or, once it's left, its last known self) attacked this combat (`AttackedOrBlockedThisCombatWatcher`)
fn attacked(game: &Game, _you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    if o.zone == Zone::Battlefield {
        return o.attacking.is_some();
    }
    game.attacked_this_turn.iter().any(|(a, _)| *a == source)
}

/// The Wolf (the trigger's pointer, kept as `entering` since the target replaces it) must be blocked this combat by
/// the target creature, if able (`RequirementEffect.mustBlockAttacker`, `Duration.EndOfCombat`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(wolf), Some(blocker)) = (cx.item.entering, cx.obj(0)) else { return Some(()) };
    if game.obj(wolf).zone != Zone::Battlefield || game.obj(blocker).zone != Zone::Battlefield {
        return Some(());
    }
    game.requirements.push(Requirement {
        kind: RequirementKind::MustBlock,
        obj: (blocker, game.obj(blocker).zcc),
        attacker: Some((wolf, game.obj(wolf).zcc)),
        end_of_combat: true,
    });
    Some(())
}

//! Killian, Ink Duelist: "Lifelink. Menace. Spells you cast that target a creature cost {2} less to cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("KillianInkDuelist.KillianInkDuelistEffect", CostKind::Reduce, your_spell, two_less_if_creature)];

/// a spell you cast
fn your_spell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller
}

/// being cast, a creature among its chosen targets (`getAllSelectedTargets`); while what's playable is checked
/// (`inCheckPlayableState`), among its possible targets (`getAllPossibleTargets`): `CardUtil.reduceCost(…, 2)`
fn two_less_if_creature(game: &Game, _: &StaticCx, of: &CostCx, cost: &mut ManaCost) {
    let creature = |o: ObjId| game.obj(o).zone == Zone::Battlefield && game.has_type(o, CardType::Creature);
    let targets_one = if let Some(item) = game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_none()) {
        item.targets.iter().flatten().any(|t| matches!(t, Target::Obj(o) if creature(*o)))
    } else {
        let spell = &of.face.spell;
        let mut targets: Vec<TargetDef> = Vec::new();
        for k in 0..spell.modes.len().max(1) {
            targets.extend(spell.mode(k).1.iter().cloned());
        }
        targets.iter().any(|def| possible_targets(game, of.seat, def, Some(of.card)).1.into_iter().any(creature))
    };
    if targets_one {
        cost.generic = cost.generic.saturating_sub(2);
    }
}

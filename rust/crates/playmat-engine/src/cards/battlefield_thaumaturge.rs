//! Battlefield Thaumaturge: "Each instant and sorcery spell you cast costs {1} less to cast for each creature it
//! targets. Heroic — Whenever you cast a spell that targets Battlefield Thaumaturge, Battlefield Thaumaturge gains
//! hexproof until end of turn."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "BattlefieldThaumaturge.BattlefieldThaumaturgeSpellsCostReductionEffect",
    CostKind::Reduce,
    your_instant_or_sorcery,
    reduce,
)];

/// an instant or sorcery spell you cast
fn your_instant_or_sorcery(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && (of.face.is(CardType::Instant) || of.face.is(CardType::Sorcery))
}

/// Being cast: {1} less for each different creature among its chosen targets (`getAllSelectedTargets`); while
/// what's playable is checked (`inCheckPlayableState`): the most creatures any one of its targets could take
/// (`getMaxPossibleTargetCreatures`, each target on its own). `CardUtil.reduceCost`: generic mana only
fn reduce(game: &Game, _: &StaticCx, of: &CostCx, cost: &mut ManaCost) {
    let creature = |o: ObjId| game.obj(o).zone == Zone::Battlefield && game.has_type(o, CardType::Creature);
    let n = if let Some(item) = game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_none()) {
        let mut seen: Vec<ObjId> = Vec::new();
        for t in item.targets.iter().flatten() {
            if let Target::Obj(o) = t {
                if creature(*o) && !seen.contains(o) {
                    seen.push(*o);
                }
            }
        }
        seen.len()
    } else {
        let spell = &of.face.spell;
        let mut most = 0;
        for k in 0..spell.modes.len().max(1) {
            for def in spell.mode(k).1.iter() {
                let possible =
                    possible_targets(game, of.seat, def, Some(of.card)).1.into_iter().filter(|o| creature(*o)).count();
                most = most.max(possible.min(target_counts(def).1));
            }
        }
        most
    };
    cost.generic = cost.generic.saturating_sub(n as u32);
}

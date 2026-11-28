//! Kopala, Warden of Waves: "Spells your opponents cast that target a Merfolk you control cost {2} more to cast.
//! Abilities your opponents activate that target a Merfolk you control cost {2} more to activate."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::cost(
        "KopalaWardenOfWaves.KopalaWardenOfWavesCostModificationEffect1",
        CostKind::Increase,
        spell_targets_merfolk,
        two_more,
    ),
    Handler::cost(
        "KopalaWardenOfWaves.KopalaWardenOfWavesCostModificationEffect2",
        CostKind::Increase,
        ability_targets_merfolk,
        two_more,
    ),
];

/// `isTargetCompatible`: a Merfolk you control
fn compatible(game: &Game, cx: &StaticCx, o: ObjId) -> bool {
    game.obj(o).controller == cx.controller && game.has_subtype(o, "Merfolk")
}

/// `isAbilityCompatible`: an opponent's; being cast or activated, a Merfolk you control among its chosen targets;
/// while what's playable is checked, among its possible targets, and no other permanent there (it could target
/// that instead)
fn targets_merfolk(game: &Game, cx: &StaticCx, of: &CostCx, targets: &[TargetDef]) -> bool {
    if !cx.opponent(of.seat) {
        return false;
    }
    let spell = of.ability.is_none();
    if let Some(item) = game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_none() == spell) {
        return item.targets.iter().flatten().any(|t| matches!(t, Target::Obj(o) if compatible(game, cx, *o)));
    }
    let mut all: Vec<ObjId> = Vec::new();
    for def in targets {
        let (_, objs) = possible_targets(game, of.seat, def, Some(of.card));
        for o in objs {
            if !all.contains(&o) {
                all.push(o);
            }
        }
    }
    let permanents: Vec<ObjId> = all.into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    !permanents.iter().any(|o| !compatible(game, cx, *o)) && permanents.iter().any(|o| compatible(game, cx, *o))
}

fn spell_targets_merfolk(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if of.ability.is_some() {
        return false;
    }
    let spell = &of.face.spell;
    let mut targets: Vec<TargetDef> = Vec::new();
    for k in 0..spell.modes.len().max(1) {
        targets.extend(spell.mode(k).1.iter().cloned());
    }
    targets_merfolk(game, cx, of, &targets)
}

fn ability_targets_merfolk(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let Some(a) = of.ability else { return false };
    let Some((_, _, targets, _, _)) = a.activated() else { return false };
    targets_merfolk(game, cx, of, targets)
}

fn two_more(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 2;
}

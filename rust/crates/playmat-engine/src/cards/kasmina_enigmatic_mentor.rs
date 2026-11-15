//! Kasmina, Enigmatic Mentor: "Spells your opponents cast that target a creature or planeswalker you control cost
//! {2} more to cast. −2: Create a 2/2 blue Wizard creature token. Draw a card, then discard a card."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "KasminaEnigmaticMentor.KasminaEnigmaticMentorCostModificationEffect",
    CostKind::Increase,
    targets_yours,
    two_more,
)];

/// `isTargetCompatible`: a creature or planeswalker you control
fn compatible(game: &Game, cx: &StaticCx, o: ObjId) -> bool {
    game.obj(o).controller == cx.controller
        && (game.has_type(o, CardType::Creature) || game.has_type(o, CardType::Planeswalker))
}

/// An opponent's spell; being cast, a creature or planeswalker you control among its chosen targets
/// (`getAllSelectedTargets`); while what's playable is checked, among its possible targets, with no other permanent
/// there (`getAllPossibleTargets`: it could target that instead)
fn targets_yours(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if of.ability.is_some() || !cx.opponent(of.seat) {
        return false;
    }
    if let Some(item) = game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_none()) {
        return item.targets.iter().flatten().any(|t| matches!(t, Target::Obj(o) if compatible(game, cx, *o)));
    }
    let spell = &of.face.spell;
    let mut all: Vec<ObjId> = Vec::new();
    for k in 0..spell.modes.len().max(1) {
        for def in spell.mode(k).1.iter() {
            for o in possible_targets(game, of.seat, def, Some(of.card)).1 {
                if !all.contains(&o) {
                    all.push(o);
                }
            }
        }
    }
    let permanents: Vec<ObjId> = all.into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    !permanents.iter().any(|o| !compatible(game, cx, *o)) && permanents.iter().any(|o| compatible(game, cx, *o))
}

fn two_more(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 2;
}

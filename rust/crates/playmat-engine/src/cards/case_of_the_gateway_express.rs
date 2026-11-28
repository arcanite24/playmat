//! Case of the Gateway Express: "When this Case enters, choose target creature you don't control. Each creature you
//! control deals 1 damage to that creature. To solve — Three or more creatures attacked this turn. Solved — Creatures
//! you control get +1/+0."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("CaseOfTheGatewayExpress.CaseOfTheGatewayExpressEffect", apply),
    Handler::condition("CaseOfTheGatewayExpress.CaseOfTheGatewayExpressCondition", three_attacked),
];

/// Each creature you control, in battlefield order, deals 1 damage to the target
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|c| game.obj(*c).controller == you && game.has_type(*c, CardType::Creature))
        .collect();
    for c in creatures {
        if game.obj(o).zone != Zone::Battlefield {
            break;
        }
        deal_damage(game, c, Target::Obj(o), 1, false);
    }
    Some(())
}

/// `AttackedThisTurnWatcher.getAttackedThisTurnCreatures().size() >= 3`: the creatures that attacked this turn,
/// wherever they are now
fn three_attacked(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.attacked_this_turn.len() >= 3
}

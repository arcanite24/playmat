//! Scouring Sands: "Scouring Sands deals 1 damage to each creature your opponents control. Scry 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ScouringSands.ScouringSandsDamageEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller != you && game.has_type(*o, CardType::Creature))
        .collect();
    for o in creatures {
        if game.obj(o).zone == Zone::Battlefield {
            deal_damage(game, cx.source_id(), Target::Obj(o), 1, false);
        }
    }
    Some(())
}

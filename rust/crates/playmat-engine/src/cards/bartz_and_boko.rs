//! Bartz and Boko: "When Bartz and Boko enters, each other Bird you control deals damage equal to its power to target
//! creature an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BartzAndBoko.BartzAndBokoEffect", apply)];

/// Each other Bird you control, in battlefield order, as the source of its own damage
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    let you = cx.controller();
    let me = cx.source_id();
    let birds: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != me && game.obj(*o).controller == you && game.has_subtype(*o, "Bird"))
        .collect();
    for bird in birds {
        if game.obj(target).zone != Zone::Battlefield || game.obj(bird).zone != Zone::Battlefield {
            continue;
        }
        let power = game.power_toughness(bird).0;
        if power > 0 {
            deal_damage(game, bird, Target::Obj(target), power, false);
        }
    }
    Some(())
}

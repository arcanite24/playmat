//! The Bears of Littjara, chapter III: "Choose up to one target creature or planeswalker. Each creature with power 4
//! or greater you control deals damage equal to its power to that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheBearsOfLittjara.TheBearsOfLittjaraEffect", apply)];

/// with the target still there, each creature you control (battlefield order) with power 4 or more deals its power
/// to it
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for c in creatures {
        if game.obj(target).zone != Zone::Battlefield || game.obj(c).zone != Zone::Battlefield {
            continue;
        }
        let power = game.power_toughness(c).0;
        if power >= 4 {
            deal_damage(game, c, Target::Obj(target), power, false);
        }
    }
    Some(())
}

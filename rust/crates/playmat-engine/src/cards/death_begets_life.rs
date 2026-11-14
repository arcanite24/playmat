//! Death Begets Life: "Destroy all creatures and enchantments. Draw a card for each permanent destroyed this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeathBegetsLife.DeathBegetsLifeEffect", apply)];

/// Each creature or enchantment in battlefield order destroyed (one that leaves the battlefield counts), then the
/// draws
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let all: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Enchantment))
        .collect();
    let mut destroyed = 0;
    for o in all {
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let zcc = game.obj(o).zcc;
        destroy(game, o);
        if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
            destroyed += 1;
        }
    }
    if destroyed > 0 {
        draw(game, cx.controller(), destroyed);
    }
    Some(())
}

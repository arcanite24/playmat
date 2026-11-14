//! Homing Lightning: "Homing Lightning deals 4 damage to target creature and each other creature with the same name
//! as that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HomingLightning.HomingLightningEffect", apply)];

/// every creature with the target's name, in battlefield order (a face-down one, with no name, only itself)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let name = game.obj(o).name().to_string();
    let nameless = game.obj(o).face_down || name.is_empty();
    let hit: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|c| {
            game.has_type(*c, CardType::Creature)
                && if nameless { *c == o } else { !game.obj(*c).face_down && game.obj(*c).name() == name }
        })
        .collect();
    for c in hit {
        deal_damage(game, cx.source_id(), Target::Obj(c), 4, false);
    }
    Some(())
}

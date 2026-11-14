//! Moonlight Hunt: "Choose target creature you don't control. Each creature you control that's a Wolf or Werewolf
//! deals damage equal to its power to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MoonlightHunt.MoonlightHuntEffect", apply)];

/// Each Wolf or Werewolf creature you control, in battlefield order, deals damage equal to its power to the target
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let wolves: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you
                && game.has_type(*o, CardType::Creature)
                && (game.has_subtype(*o, "Wolf") || game.has_subtype(*o, "Werewolf"))
        })
        .collect();
    for w in wolves {
        let power = game.power_toughness(w).0;
        deal_damage(game, w, Target::Obj(t), power, false);
    }
    Some(())
}

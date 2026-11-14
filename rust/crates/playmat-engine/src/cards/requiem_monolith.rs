//! Requiem Monolith: "{T}: Until end of turn, target creature gains \"Whenever this creature is dealt damage, you draw
//! that many cards and lose that much life.\" That creature's controller may have this artifact deal 1 damage to it.
//! Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RequiemMonolith.RequiemMonolithEffect", apply)];

/// the creature's controller is asked, then 1 damage from the artifact
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(o).controller;
    let message = format!("Have this artifact deal 1 damage to {}?", log::object(game, o));
    if choose_use(game, d, p, &message)? {
        deal_damage(game, cx.source_id(), Target::Obj(o), 1, false);
    }
    Some(())
}

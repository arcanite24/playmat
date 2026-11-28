//! `DamageAttachedControllerEffect(n)`: "this deals N damage to that creature's controller" (Gremlin Infestation,
//! Impending Doom)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageAttachedControllerEffect", apply).args(&["int"])];

/// the source (or as it last was) and what it's attached to (or was, as it last was on the battlefield): that
/// permanent's controller is dealt N damage by the source
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_int(0)?;
    let src = game.obj(cx.source_id());
    let to = if src.zone == Zone::Battlefield { src.attached_to } else { src.lki_attached_to };
    let (o, _) = to?;
    let p = game.obj(o).controller;
    deal_damage(game, cx.source_id(), Target::Player(p), n, false);
    Some(())
}

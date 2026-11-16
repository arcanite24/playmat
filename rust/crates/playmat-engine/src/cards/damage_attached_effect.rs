//! `DamageAttachedEffect(n)` / `(n, name)`: "it deals N damage to enchanted creature" (Tilonalli's Crown)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageAttachedEffect", apply).args(&["int", "text"])];

/// the source (or as it last was) and the permanent it's attached to, still on the battlefield: N damage to it from
/// the source (damage to a permanent that already left changes nothing)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_int(0)?;
    let src = game.obj(cx.source_id());
    let to = if src.zone == Zone::Battlefield { src.attached_to } else { src.lki_attached_to };
    let (o, zcc) = to?;
    if game.obj(o).zone != Zone::Battlefield || game.obj(o).zcc != zcc {
        return Some(());
    }
    deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
    Some(())
}

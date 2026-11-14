//! Hexgold Slash: "Hexgold Slash deals 2 damage to target creature. If that creature has toxic, Hexgold Slash deals 4
//! damage to that creature instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HexgoldSlash.HexgoldSlashEffect", apply)];

/// 4 damage to a creature with a toxic ability (its own or granted), else 2
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let toxic = |a: &AbilityDef| matches!(a, AbilityDef::ToxicAbility { .. });
    let has = game.obj(t).def.abilities.iter().any(toxic) || game.granted_instances(t).iter().any(toxic);
    deal_damage(game, cx.source_id(), Target::Obj(t), if has { 4 } else { 2 }, false);
    Some(())
}

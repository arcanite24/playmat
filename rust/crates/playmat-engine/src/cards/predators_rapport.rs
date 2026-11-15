//! Predator's Rapport: "Choose target creature you control. You gain life equal to that creature's power plus its
//! toughness."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("PredatorsRapport.TargetPermanentPowerPlusToughnessCount", power_plus_toughness)];

/// `game.getPermanent(sourceAbility.getFirstTarget())`: its power plus its toughness, while it's on the battlefield
fn power_plus_toughness(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(item) = game.resolving_item.as_ref() else { return 0 };
    let first = item.targets.first().and_then(|t| t.first().copied());
    let zcc = item.target_zcc.first().and_then(|z| z.first().copied());
    let (Some(Target::Obj(o)), Some(zcc)) = (first, zcc) else { return 0 };
    let obj = game.obj(o);
    if obj.zone != crate::state::Zone::Battlefield || obj.zcc != zcc {
        return 0;
    }
    let (p, t) = game.power_toughness(o);
    p.saturating_add(t)
}

//! `TargetPermanentPowerCount`: the power of the effect's first target permanent, or of it as it last was on the
//! battlefield (Onward, Arni, Renowned Champion's entering creature)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("TargetPermanentPowerCount", target_power)];

/// `getFirstTargetPermanentOrLKI(game, sourceAbility).getPower()`
fn target_power(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(item) = game.resolving_item.as_ref() else { return 0 };
    let Some(Target::Obj(o)) = item.targets.first().and_then(|ts| ts.first()).copied() else { return 0 };
    let zcc = item.target_zcc.first().and_then(|zs| zs.first()).copied();
    let p = game.obj(o);
    if p.zone == Zone::Battlefield && zcc.is_none_or(|z| z == p.zcc) {
        game.power_toughness(o).0
    } else {
        p.power_lki.unwrap_or(0)
    }
}

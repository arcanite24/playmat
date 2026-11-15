//! Dormant Grove: "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. Then
//! if that creature has toughness 6 or greater, transform Dormant Grove."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("DormantGrove.DormantGroveCondition", toughness_six)];

/// the resolving ability's first target, still that permanent, has toughness 6 or greater
fn toughness_six(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(item) = game.resolving_item.as_ref() else { return false };
    let Some(Target::Obj(o)) = item.targets.first().and_then(|t| t.first()).copied() else { return false };
    let zcc = item.target_zcc.first().and_then(|z| z.first()).copied();
    let p = game.obj(o);
    p.zone == Zone::Battlefield && Some(p.zcc) == zcc && game.power_toughness(o).1 >= 6
}

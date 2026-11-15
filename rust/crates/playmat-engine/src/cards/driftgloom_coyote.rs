//! Driftgloom Coyote: "When Driftgloom Coyote enters, exile target creature an opponent controls until Driftgloom
//! Coyote leaves the battlefield. If that creature had power 2 or less, put a +1/+1 counter on Driftgloom Coyote."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("DriftgloomCoyote.TargetObjectMatchesFilterCondition", power_two_or_less)];

/// `TargetObjectMatchesFilterCondition(power 2 or less)`: `game.getObject(source.getFirstTarget())`, the creature
/// where it is now (exiled, its card's own power; still on the battlefield, its power)
fn power_two_or_less(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(Target::Obj(o)) = game.resolving_item.as_ref().and_then(|i| i.targets.first()?.first().copied()) else {
        return false;
    };
    let obj = game.obj(o);
    let power = match obj.zone {
        crate::state::Zone::Battlefield => game.power_toughness(o).0,
        // a token that left is gone (`getObject` finds nothing)
        _ if obj.token => return false,
        _ => obj.def.power,
    };
    power <= 2
}

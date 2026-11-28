//! Driftgloom Coyote: "When Driftgloom Coyote enters, exile target creature an opponent controls until Driftgloom
//! Coyote leaves the battlefield. If that creature had power 2 or less, put a +1/+1 counter on Driftgloom Coyote."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("DriftgloomCoyote.DriftgloomCoyoteCondition", had_power_two_or_less)];

/// `game.getPermanentOrLKIBattlefield(source.getFirstTarget())` with power 2 or less: the creature still on the
/// battlefield, or as it last was there (it's exiled by then)
fn had_power_two_or_less(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(Target::Obj(o)) = game.resolving_item.as_ref().and_then(|i| i.targets.first()?.first().copied()) else {
        return false;
    };
    let obj = game.obj(o);
    let power = if obj.zone == crate::state::Zone::Battlefield { Some(game.power_toughness(o).0) } else { obj.power_lki };
    power.is_some_and(|p| p <= 2)
}

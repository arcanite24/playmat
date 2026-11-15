//! Driftgloom Coyote: "When Driftgloom Coyote enters, exile target creature an opponent controls until Driftgloom
//! Coyote leaves the battlefield. If that creature had power 2 or less, put a +1/+1 counter on Driftgloom Coyote."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("DriftgloomCoyote.TargetObjectMatchesFilterCondition", power_two_or_less)];

/// `TargetObjectMatchesFilterCondition(power 2 or less)`: `game.getObject(source.getFirstTarget())` matched against a
/// `FilterPermanent`, which matches only a permanent: the creature still on the battlefield with power 2 or less (an
/// exiled one never matches, so the counter comes only when the exile didn't happen)
fn power_two_or_less(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(Target::Obj(o)) = game.resolving_item.as_ref().and_then(|i| i.targets.first()?.first().copied()) else {
        return false;
    };
    game.obj(o).zone == crate::state::Zone::Battlefield && game.power_toughness(o).0 <= 2
}

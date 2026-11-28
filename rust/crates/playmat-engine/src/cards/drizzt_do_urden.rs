//! Drizzt Do'Urden: "Whenever a creature dies, if it had power greater than Drizzt's power, put a number of +1/+1
//! counters on Drizzt equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("DrizztDoUrden.DrizztDoUrdenCondition", stronger),
    Handler::effect("DrizztDoUrden.DrizztDoUrdenEffect", apply),
];

/// Drizzt's power, on the battlefield or as it last was there
fn own_power(game: &Game, source: ObjId) -> Option<i32> {
    let o = game.obj(source);
    if o.zone == Zone::Battlefield {
        Some(game.power_toughness(source).0)
    } else {
        o.power_lki
    }
}

/// the creature that died had greater power than Drizzt
fn stronger(game: &Game, _: Seat, source: ObjId) -> bool {
    let Some(item) = super::liberator_urzas_battlethopter::trigger_item(game, source) else { return true };
    let Some((died, _)) = item.died else { return false };
    let Some(power) = own_power(game, source) else { return false };
    game.obj(died).power_lki.unwrap_or(0) > power
}

/// the difference as +1/+1 counters on Drizzt, while it's on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let src = cx.source_id();
    let Some(power) = own_power(game, src) else { return Some(()) };
    let n = (game.obj(died).power_lki.unwrap_or(0) - power).max(0);
    if n > 0 && cx.source_on_battlefield(game) {
        put_counters(game, src, "+1/+1", n, cx.controller());
    }
    Some(())
}

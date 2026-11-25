//! Cheering Crowd: "At the beginning of each player's first main phase, that player may put a +1/+1 counter on this
//! creature. If they do, they add {C} for each counter on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CheeringCrowd.CheeringCrowdDoIfPaidEffect", apply)];

/// `DoIfCostPaid` paid by "that player" (`getPayingPlayer`: the target pointer, the active player): its own question,
/// then `PutCountersSourceCost` (a counter they put, through the counter replacements, while the Crowd is still the permanent), then {C} for each +1/+1
/// counter on it into their pool
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(player) = cx.player(0).or(game.active) else { return Some(()) };
    let src = cx.source_id();
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let message = format!("Put a +1/+1 counter on {} and add {{C}} for each counter on it?", game.obj(src).name());
    if !choose_use(game, d, player, &message)? {
        return Some(());
    }
    // (`addCounters`: the replacements see it, Hardened Scales' extra one)
    put_counters(game, src, "+1/+1", 1, player);
    let n = game.obj(src).counter("+1/+1").max(0) as u32;
    game.player_mut(player).mana_pool.colorless += n;
    Some(())
}

//! `MoveCountersFromSourceToTargetEffect`: "move a +1/+1 counter from {this} onto target creature you control"
//! (Costume Closet), of a counter type and amount

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MoveCountersFromSourceToTargetEffect", apply).args(&["counter_type", "text"])];

/// both still permanents and not the same one, the source with enough of those counters: the target gets them,
/// then the source loses them
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let name = cx.args.first().map_or("+1/+1", |a| a.as_str());
    let amount: i32 = cx.args.get(1).and_then(|a| a.trim().parse().ok()).unwrap_or(1);
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let src = cx.source_id();
    let Some(to) = cx.obj(0) else { return Some(()) };
    if to == src || game.obj(to).zone != Zone::Battlefield || game.obj(src).counter(name) < amount {
        return Some(());
    }
    if add_counters_by(game, to, name, amount, Some(cx.controller())) <= 0 {
        return Some(());
    }
    let o = game.obj_mut(src);
    o.add_counters(name, -amount);
    o.counters.retain(|(_, k)| *k > 0);
    Some(())
}

//! `GainControlAllEffect(Duration.Custom, filter)`: "gain control of all <filter>" for good (Gruul Charm's "all
//! permanents you own", Tribute to Horobi's back face)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GainControlAllEffect", apply).args(&["one_of:Duration.Custom", "filter"])];

/// each permanent of the filter as it resolves gets its own `GainControlTargetEffect` (a fixed target), in
/// battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let all: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| game.filter_matches(&filter, you, Some(cx.source_id()), *b))
        .collect();
    for o in all {
        let e = EffectDef::GainControlTargetEffect { duration: "Custom".into() };
        cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    }
    Some(())
}

//! `LoseAllAbilitiesAllEffect(filter, Duration.EndOfTurn)`: "all creatures lose all abilities until end of turn"
//! (Final Showdown, Day of Black Sun). Java locks in the permanents of the filter as it resolves (`affectedObjectList`),
//! so each one loses its abilities as `LoseAllAbilitiesTargetEffect` would.

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("LoseAllAbilitiesAllEffect", apply).args(&["filter", "one_of:Duration.EndOfTurn"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let filter = cx.filters.first()?;
    let you = cx.controller();
    let hit: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.filter_matches(filter, you, Some(cx.source_id()), *o))
        .map(Target::Obj)
        .collect();
    if !hit.is_empty() {
        cx.run_on(game, d, &[EffectDef::LoseAllAbilitiesTargetEffect], hit);
    }
    Some(())
}

//! `DetainAllEffect(filter)`: "detain each nonland permanent your opponents control with mana value 4 or less"
//! (Lavinia of the Tenth): the permanents of the filter as it resolves are detained (until your next turn they can't
//! attack or block and their activated abilities can't be activated), as the engine's detain does.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DetainAllEffect", apply).args(&["filter"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let objs: Vec<Target> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).zone == Zone::Battlefield && game.filter_matches(filter, you, Some(cx.source_id()), *o))
        .map(Target::Obj)
        .collect();
    if !objs.is_empty() {
        cx.run_on(game, d, &[EffectDef::EdificeOfAuthorityEffect], objs);
    }
    Some(())
}

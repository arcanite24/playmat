//! `SacrificeAllControllerEffect(filter)`: "sacrifice all Reflections you control" (The Apprentice's Folly's
//! chapter III).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SacrificeAllControllerEffect", apply).args(&["filter"])];

/// each permanent you control that matches, in battlefield order (`getAllActivePermanents(playerId)`), sacrificed
/// one by one
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let all: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == you).collect();
    for o in all {
        if game.obj(o).zone == Zone::Battlefield
            && game.obj(o).controller == you
            && game.filter_matches(filter, you, Some(cx.source_id()), o)
        {
            sacrifice(game, o);
        }
    }
    Some(())
}

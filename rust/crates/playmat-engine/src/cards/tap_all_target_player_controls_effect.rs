//! `TapAllTargetPlayerControlsEffect(filter)`: "tap all [filter] target player controls" (Kiora Bests the Sea God's
//! chapter II, Dovin, Architect of Law's −9, Torrent Elemental).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TapAllTargetPlayerControlsEffect", apply).args(&["filter"])];

/// `getAllActivePermanents(filter, player, game)`: the target player's permanents of the filter, in battlefield
/// order, each tapped (`tap` does nothing to a tapped one)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let filter = cx.filters.first()?;
    let all: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.filter_matches(filter, p, Some(cx.source_id()), *o))
        .collect();
    for o in all {
        if !game.obj(o).tapped {
            tap(game, o);
        }
    }
    Some(())
}

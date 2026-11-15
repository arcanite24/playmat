//! `EnterUntappedAllEffect(filter)`: "Lands you control enter untapped." (Spelunking, The Wandering Minstrel)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EnterUntappedAllEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)
.args(&["filter"])];

/// the entering permanent matches the filter, for the source's controller
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target.is_some_and(|id| cx.matches(game, 0, id))
}

/// `target.setTapped(false)`
fn replace(game: &mut Game, _cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(id) = ev.target {
        game.obj_mut(id).tapped = false;
    }
    false
}

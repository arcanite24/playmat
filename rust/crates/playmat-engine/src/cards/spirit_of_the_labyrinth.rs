//! Spirit of the Labyrinth: "Each player can't draw more than one card each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "SpiritOfTheLabyrinth.SpiritOfTheLabyrinthEffect",
    &[EventType::DrawCard],
    drew_this_turn,
    stop,
)];

/// `SpiritOfTheLabyrinthWatcher`: the player already drew a card this turn (`DREW_CARD`)
fn drew_this_turn(game: &Game, _: &StaticCx, ev: &Event) -> bool {
    game.player(ev.player).drawn >= 1
}

/// a rule-modifying effect on `DRAW_CARD`: the draw doesn't happen
fn stop(_: &mut Game, _: &StaticCx, _: &mut Event) -> bool {
    true
}

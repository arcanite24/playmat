//! Spider-UK: "Web-slinging {2}{W}. At the beginning of your end step, if two or more creatures entered the
//! battlefield under your control this turn, you draw a card and gain 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SpiderUK.SpiderUKCondition", two_entered)];

/// `SpiderUKWatcher`: two or more creatures (creatures as they entered) entered under your control this turn
fn two_entered(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.entered.iter().filter(|(c, _, creature, _)| *c == you && *creature).count() >= 2
}

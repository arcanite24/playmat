//! Lyra, Tolarian Archangel: "At the beginning of each end step, if you've drawn three or more cards this turn,
//! create a 3/3 blue Angel creature token with flying."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("LyraTolarianArchangel.LyraTolarianArchangelCondition", drew_three)];

/// `CardsAmountDrawnThisTurnWatcher.getAmountCardsDrawn(you) >= 3`
fn drew_three(game: &Game, you: Seat, _: ObjId) -> bool {
    game.player(you).drawn >= 3
}

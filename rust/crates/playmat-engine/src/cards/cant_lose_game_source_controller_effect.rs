//! `CantLoseGameSourceControllerEffect`: "You can't lose the game and your opponents can't win the game." (Herald of
//! Eternal Dawn; the ability The Book of Exalted Deeds gives an Angel)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("CantLoseGameSourceControllerEffect", Rule::CantLoseOrWin(you_or_an_opponent))];

/// `LOSES` for the source's controller, `WINS` for an opponent of theirs
fn you_or_an_opponent(_: &Game, cx: &StaticCx, seat: Seat, wins: bool) -> bool {
    (seat == cx.controller) != wins
}

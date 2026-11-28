//! Toph, the Blind Bandit: "Toph's power is equal to the number of +1/+1 counters on lands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("TophTheBlindBandit.TophTheBlindBanditValue", counters_on_lands)];

fn counters_on_lands(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == you && game.has_type(**b, CardType::Land))
        .map(|b| game.obj(*b).counter("+1/+1"))
        .sum()
}

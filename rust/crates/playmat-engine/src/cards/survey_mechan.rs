//! Survey Mechan: "{10}, Sacrifice this creature: … This ability costs {X} less to activate, where X is the number
//! of differently named lands you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("SurveyMechan.DifferentlyNamedPermanentCount", differently_named_lands)];

/// `DifferentlyNamedPermanentCount(FILTER_CONTROLLED_PERMANENT_LANDS)`: the distinct names among the lands you
/// control
fn differently_named_lands(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut names: Vec<&str> = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
        .map(|o| game.obj(*o).name())
        .filter(|n| !n.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();
    names.len() as i32
}

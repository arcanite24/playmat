//! Sardian Cliffstomper: "As long as it's your turn and you control four or more Mountains, this creature gets
//! +X/+0, where X is the number of Mountains you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("SardianCliffstomper.SardianCliffstomperCondition", your_turn_four_mountains)];

/// your turn, and four or more Mountains you control
fn your_turn_four_mountains(game: &Game, you: Seat, _: ObjId) -> bool {
    game.active == Some(you)
        && game
            .battlefield
            .iter()
            .filter(|b| game.obj(**b).controller == you && game.has_subtype(**b, "Mountain"))
            .count()
            >= 4
}

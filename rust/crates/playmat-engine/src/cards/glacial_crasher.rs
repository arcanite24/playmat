//! Glacial Crasher: "Trample. Glacial Crasher can't attack unless there is a Mountain on the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "GlacialCrasher.GlacialCrasherEffect",
    applies,
    &[Restrict::CantAttack(no_attack)],
)];

/// the source itself, while no land on the battlefield is a Mountain
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
        && !game.battlefield.iter().any(|c| game.has_type(*c, CardType::Land) && game.has_subtype(*c, "Mountain"))
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

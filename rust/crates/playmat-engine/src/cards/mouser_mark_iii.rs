//! Mouser Mark III: "This creature can't attack unless you control another artifact."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("MouserMarkIII.MouserMarkIIIEffect", applies, &[Restrict::CantAttack(no_attack)])];

/// the source itself, while its controller controls no other artifact
fn applies(game: &Game, cx: &StaticCx, permanent: ObjId) -> bool {
    permanent == cx.source
        && !game.battlefield.iter().any(|c| {
            *c != cx.source && game.obj(*c).controller == cx.controller && game.has_type(*c, CardType::Artifact)
        })
}

fn no_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

//! Palisade Giant: "All damage that would be dealt to you or another permanent you control is dealt to Palisade
//! Giant instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PalisadeGiant.PalisadeGiantReplacementEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage to you, or to a permanent you control without the Giant's name (no redirection from or to other Palisade
/// Giants, `CardUtil.haveSameNames`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target.is_some_and(|t| {
            let here = game.obj(cx.source).zone == Zone::Battlefield;
            game.obj(t).zone == Zone::Battlefield
                && game.obj(t).controller == cx.controller
                && !(here && game.obj(t).name() == game.obj(cx.source).name())
        }),
    }
}

/// dealt to the Giant instead, while it's on the battlefield
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    redirect_damage(game, ev, cx.source);
    true
}

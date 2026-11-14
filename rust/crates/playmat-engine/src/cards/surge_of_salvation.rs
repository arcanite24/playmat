//! Surge of Salvation: "You and permanents you control gain hexproof until end of turn. Prevent all damage that
//! black and/or red sources would deal to creatures you control this turn."

use super::*;
use crate::card::{CardType, Color};

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "SurgeOfSalvation.SurgeOfSalvationEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// `PreventionEffectImpl.applies` (damage that can be prevented), to a creature you control from a black or red
/// source
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false)
        && ev.target.is_some_and(|t| {
            game.obj(t).zone == Zone::Battlefield
                && game.has_type(t, CardType::Creature)
                && game.obj(t).controller == cx.controller
        })
        && ev.source.is_some_and(|s| game.colors(s).iter().any(|c| matches!(c, Color::B | Color::R)))
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}

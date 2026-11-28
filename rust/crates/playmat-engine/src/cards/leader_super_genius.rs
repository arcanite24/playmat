//! Leader, Super-Genius: "If a creature you control would connive, instead you draw a card, then that creature
//! connives."

use super::*;
use crate::card::CardType;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "LeaderSuperGenius.LeaderSuperGeniusEffect",
    &[EventType::Connive],
    applies,
    replace,
)];

/// a creature you control (`getPermanentOrLKIBattlefield`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target.is_some_and(|t| game.obj(t).controller == cx.controller && game.has_type(t, CardType::Creature))
}

/// you draw a card, and it connives as it would have
fn replace(game: &mut Game, cx: &StaticCx, _: &mut Event) -> bool {
    draw(game, cx.controller, 1);
    false
}

//! Rankle and Torbran: "Flying, first strike, haste. Whenever Rankle and Torbran deals combat damage to a player or
//! battle, choose any number — • Each player creates a Treasure token. • Each player sacrifices a creature. • If a
//! source would deal damage to a player or battle this turn, it deals that much damage plus 2 instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "RankleAndTorbran.RankleAndTorbranEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    plus_two,
)];

/// some damage to a player, or to a battle
fn applies(game: &Game, _: &StaticCx, ev: &Event) -> bool {
    if ev.amount < 1 {
        return false;
    }
    match ev.kind {
        EventType::DamagePlayer => true,
        _ => ev.target.is_some_and(|t| game.obj(t).zone == Zone::Battlefield && game.has_type(t, CardType::Battle)),
    }
}

/// `CardUtil.overflowInc(amount, 2)`
fn plus_two(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(2);
    false
}

//! The Mindskinner: "The Mindskinner can't be blocked. If a source you control would deal damage to an opponent,
//! prevent that damage and each opponent mills that many cards."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("TheMindskinner.TheMindskinnerEffect", &[EventType::DamagePlayer], applies, replace)];

/// `PreventionEffectImpl.applies` (damage that can be prevented) from a source you control to an opponent of its
/// controller
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    let owner = game.obj(src).controller;
    cx.preventable(ev, false)
        && ev.kind == EventType::DamagePlayer
        && owner == cx.controller
        && Game::whose("opponent", owner, ev.player)
}

/// the damage as it would have been, then the damage prevented and each opponent, in turn order from you, mills that
/// many cards (`millCards`)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let n = ev.amount.max(0) as usize;
    cx.prevent(ev, i32::MAX);
    for k in 1..game.seats() {
        let p = (cx.controller + k) % game.seats();
        if !Game::whose("opponent", cx.controller, p) {
            continue;
        }
        let top: Vec<ObjId> = game.player(p).library.iter().take(n).copied().collect();
        note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
        for c in top {
            game.move_to(c, Zone::Graveyard);
        }
    }
    false
}

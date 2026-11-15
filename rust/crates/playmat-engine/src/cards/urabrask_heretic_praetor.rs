//! Urabrask, Heretic Praetor: "Haste. At the beginning of your upkeep, exile the top card of your library. You may
//! play it this turn. At the beginning of each opponent's upkeep, the next time they would draw a card this turn,
//! instead they exile the top card of their library. They may play it this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "UrabraskHereticPraetor.UrabraskHereticPraetorEffect",
    &[EventType::DrawCard],
    applies,
    replace,
)];

/// the active player's draw, the first one only (`discard()` once it has replaced one)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    game.active == Some(ev.player) && cx.left(game, 1) > 0
}

/// the top card of their library exiled instead, playable by them this turn
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.set_left(game, 0);
    let player = ev.player;
    if let Some(&card) = game.player(player).library.first() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile {
            let turn = game.turn;
            game.obj_mut(card).may_play = Some((player, turn));
        }
    }
    true
}

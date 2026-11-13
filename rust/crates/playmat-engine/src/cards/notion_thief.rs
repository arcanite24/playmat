//! Notion Thief: "If an opponent would draw a card except the first one they draw in each of their draw steps,
//! instead that player skips that draw and you draw a card."

use super::*;
use crate::state::Step;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "NotionThief.NotionThiefReplacementEffect",
    &[EventType::DrawCard],
    applies,
    replace,
)];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.opponent(ev.player) {
        return false;
    }
    if game.active == Some(ev.player) && game.step == Some(Step::Draw) {
        // `CardsDrawnDuringDrawStepWatcher`
        return game.player(ev.player).drawn_in_draw_step > 0;
    }
    true
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    draw_applied(game, cx.controller, 1, &ev.applied);
    true
}

//! `DrawExceptFirstDrawTwoReplacementEffect`: "If you would draw a card except the first one you draw in each of your
//! draw steps, draw two cards instead." (Teferi's Ageless Insight, Alhammarret's Archive)

use super::*;
use crate::state::Step;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("DrawExceptFirstDrawTwoReplacementEffect", &[EventType::DrawCard], applies, replace)];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if ev.player != cx.controller {
        return false;
    }
    if game.active != Some(ev.player) || game.step != Some(Step::Draw) {
        return true;
    }
    // `CardsDrawnDuringDrawStepWatcher`
    game.player(ev.player).drawn_in_draw_step > 0
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    draw_applied(game, cx.controller, 2, &ev.applied);
    true
}

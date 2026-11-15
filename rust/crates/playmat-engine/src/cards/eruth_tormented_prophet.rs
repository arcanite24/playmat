//! Eruth, Tormented Prophet: "If you would draw a card, exile the top two cards of your library instead. You may
//! play those cards this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EruthTormentedProphet.EruthTormentedProphetEffect",
    &[EventType::DrawCard],
    applies,
    replace,
)];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

/// the top two cards into exile, playable this turn (`makeCardPlayable(…, Duration.EndOfTurn)`); the draw is
/// replaced
fn replace(game: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    let you = ev.player;
    let cards: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    let turn = game.turn;
    for c in cards {
        if game.obj(c).zone == Zone::Exile {
            game.obj_mut(c).may_play = Some((you, turn));
        }
    }
    true
}

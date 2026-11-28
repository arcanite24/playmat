//! Jace, Wielder of Mysteries: "If you would draw a card while your library has no cards in it, you win the game
//! instead." and "−8: Draw seven cards. Then if your library has no cards in it, you win the game."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "JaceWielderOfMysteries.JaceWielderOfMysteriesContinuousEffect",
        &[EventType::DrawCard],
        applies,
        replace,
    ),
    Handler::effect("JaceWielderOfMysteries.JaceWielderOfMysteriesEffect", apply),
];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let p = game.player(ev.player);
    ev.player == cx.controller && !p.lost && p.library.is_empty()
}

/// `player.won(game)` for the event's player; the draw is replaced
fn replace(game: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    win_game(game, ev.player);
    true
}

fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 7);
    if game.player(you).library.is_empty() {
        win_game(game, you);
    }
    Some(())
}

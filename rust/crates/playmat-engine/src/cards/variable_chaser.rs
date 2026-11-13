//! Variable Chaser // Arc of Fortune: "Each player may discard their hand and draw seven cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VariableChaser.ArcOfFortuneEffect", apply)];

/// Each player in range from you is asked in turn; then each who said yes, in that order, discards their hand (one
/// discard of all of it) and draws seven
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut wheelers = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        if choose_use(game, d, p, "Discard your hand and draw seven?")? {
            if log::kept(game) {
                let line = format!("{} chooses to discard their hand and draw seven", log::player(p));
                log::inform(game, line);
            }
            wheelers.push(p);
        }
    }
    for p in wheelers {
        for c in game.player(p).hand.clone() {
            discard_card(game, p, c);
        }
        game.discarded_cards();
        draw(game, p, 7);
    }
    Some(())
}

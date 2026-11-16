//! Zimone, All-Questioning: "At the beginning of your end step, if a land entered the battlefield under your control
//! this turn and you control a prime number of lands, create Primo, the Indivisible, a legendary 0/0 green and blue
//! Fractal creature token, then put that many +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ZimoneAllQuestioning.ZimoneAllQuestioningCondition", prime_lands),
    Handler::effect("ZimoneAllQuestioning.ZimoneAllQuestioningEffect", apply).tokens(&["PrimoTheIndivisibleToken"]),
];

fn lands_you_control(game: &Game, you: Seat) -> i32 {
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
        .count() as i32
}

/// `CardUtil.isPrime`
fn is_prime(n: i32) -> bool {
    n >= 2 && (2..).take_while(|k| k * k <= n).all(|k| n % k != 0)
}

/// `LandfallWatcher.landPlayed(you)` and a prime number of lands you control
fn prime_lands(game: &Game, you: Seat, _: ObjId) -> bool {
    game.player(you).lands_entered > 0 && is_prime(lands_you_control(game, you))
}

/// Primo created, then the lands you control counted and that many +1/+1 counters on each token it made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, cx.tokens.first()?, 1);
    if d.stopped() {
        return None;
    }
    let count = lands_you_control(game, you);
    if count < 1 {
        return Some(());
    }
    let made: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| !before.contains(o)).collect();
    for o in made {
        put_counters(game, o, "+1/+1", count, you);
    }
    Some(())
}

//! Ochre Jelly: "Split — When Ochre Jelly dies, if it had two or more +1/+1 counters on it, create a token that's a
//! copy of it at the beginning of the next end step. That token enters with half that many +1/+1 counters on it,
//! rounded down."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("OchreJelly.OchreJellyCondition", had_two),
    Handler::effect("OchreJelly.OchreJellyEffect", apply),
];

/// the +1/+1 counters it had as it left the battlefield (`permanentLeftBattlefield`)
fn had(game: &Game, source: ObjId) -> i32 {
    game.obj(source).lki_counters.iter().find(|(n, _)| n == "+1/+1").map_or(0, |(_, k)| *k)
}

fn had_two(game: &Game, _you: Seat, source: ObjId) -> bool {
    had(game, source) >= 2
}

/// `CreateTokenCopyTargetEffect(P1P1, n / 2).setSavedPermanent(permanent)`: a copy of it as it last was, wherever
/// the card is now, with half its counters
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let of = cx.source_id();
    let counters = had(game, of) / 2;
    let made = token_copy_modified(game, d, of, you, 1, &|_| {});
    if d.stopped() {
        return None;
    }
    for t in made {
        if counters > 0 && game.obj(t).zone == Zone::Battlefield {
            add_counters_by(game, t, "+1/+1", counters, Some(you));
        }
    }
    Some(())
}

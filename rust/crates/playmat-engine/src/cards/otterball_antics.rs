//! Otterball Antics: "Create a 1/1 blue and red Otter creature token with prowess. If this spell was cast from
//! anywhere other than your hand, put a +1/+1 counter on that creature."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("OtterballAntics.OtterballAnticsEffect", apply).tokens(&["OtterProwessToken"])];

/// the token; the spell cast from elsewhere than a hand (flashback): a +1/+1 counter on each token made
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let before: Vec<ObjId> = game.battlefield.to_vec();
    cx.create_tokens(game, d, &cx.tokens[0], 1);
    if d.stopped() {
        return None;
    }
    if !matches!(game.obj(cx.source_id()).cast_from, Some(z) if z != Zone::Hand) {
        return Some(());
    }
    let made: Vec<ObjId> =
        game.battlefield.iter().copied().filter(|o| !before.contains(o) && game.obj(*o).token).collect();
    for t in made {
        put_counters(game, t, "+1/+1", 1, you);
    }
    Some(())
}

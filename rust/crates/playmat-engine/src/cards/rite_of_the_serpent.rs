//! Rite of the Serpent: "Destroy target creature. If that creature had a +1/+1 counter on it, create a 1/1 green
//! Snake creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RiteOfTheSerpent.RiteOfTheSerpentEffect", apply).tokens(&["SnakeToken"])];

/// The creature's +1/+1 counters now, or as it last was on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let had = if game.obj(o).zone == Zone::Battlefield {
        game.obj(o).counter("+1/+1") > 0
    } else {
        game.obj(o).lki_counters.iter().any(|(n, k)| n == "+1/+1" && *k > 0)
    };
    if had {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}

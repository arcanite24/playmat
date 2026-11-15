//! Polukranos, Unchained: "Polukranos enters with six +1/+1 counters on it. It escapes with twelve +1/+1 counters on
//! it instead. If damage would be dealt to Polukranos while it has a +1/+1 counter on it, prevent that damage and
//! remove that many +1/+1 counters from it. {1}{B}{G}: Polukranos fights another target creature. Escape—{4}{B}{G},
//! Exile six other cards from your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PolukranosUnchained.PolukranosUnchainedEffect", apply)];

/// As it enters: twelve +1/+1 counters if it was cast with escape (`EscapeAbility`, this object), else six
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.source_id();
    let n = if game.obj(id).escaped { 12 } else { 6 };
    add_counters_by(game, id, "+1/+1", n, Some(cx.controller()));
    Some(())
}

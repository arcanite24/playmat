//! Alien Invasion: "At the beginning of combat on your turn, create a 1/1 red Alien creature token with haste and
//! 'This token attacks each combat if able.' Put a +1/+1 counter on it for each invasion counter on this
//! enchantment, then put an invasion counter on this enchantment."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AlienInvasion.AlienInvasionEffect", apply).tokens(&["AlienHasteToken"])];

/// the invasion counters (the source's, or as it last was), the token, that many +1/+1 counters on it, then an
/// invasion counter on the source while it's still there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let on_battlefield = cx.source_on_battlefield(game);
    let invasion = if on_battlefield {
        game.obj(src).counter("invasion")
    } else if game.obj(src).zcc == cx.source.1 + 1 {
        game.obj(src).lki_counters.iter().filter(|(c, _)| c == "invasion").map(|(_, n)| *n).sum()
    } else {
        return Some(());
    };
    let before = game.objs.len();
    cx.create_tokens(game, d, cx.tokens.first()?, 1);
    if d.stopped() {
        return None;
    }
    if invasion > 0 {
        let made: Vec<ObjId> = (before..game.objs.len())
            .map(|i| ObjId(i as u32))
            .filter(|o| game.obj(*o).token && game.obj(*o).zone == Zone::Battlefield)
            .collect();
        for t in made {
            add_counters_by(game, t, "+1/+1", invasion, Some(cx.controller()));
        }
    }
    if cx.source_on_battlefield(game) {
        add_counters_by(game, src, "invasion", 1, Some(cx.controller()));
    }
    Some(())
}

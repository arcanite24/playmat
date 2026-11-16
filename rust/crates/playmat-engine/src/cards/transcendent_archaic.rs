//! Transcendent Archaic: "Vigilance. Converge — When this creature enters, you may draw X cards, where X is the
//! number of colors of mana spent to cast this spell. If you draw one or more cards this way, discard two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TranscendentArchaic.TranscendentArchaicEffect", apply)];

/// `ColorsOfManaSpentToCastCount`: none, nothing; else draw that many, then discard two
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.obj(cx.source_id()).spent_colors.iter().filter(|n| **n > 0).count();
    if n == 0 {
        return Some(());
    }
    draw(game, you, n);
    discard(game, d, you, 2);
    if d.stopped() {
        return None;
    }
    Some(())
}

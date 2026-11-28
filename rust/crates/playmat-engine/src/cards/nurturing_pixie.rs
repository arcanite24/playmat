//! Nurturing Pixie: "When Nurturing Pixie enters the battlefield, return up to one target non-Faerie, nonland
//! permanent you control to its owner's hand. If a permanent was returned this way, put a +1/+1 counter on Nurturing
//! Pixie."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NurturingPixie.NurturingPixieEffect", apply)];

/// With the target still there, it goes to its owner's hand and the source (if it still exists) gets the counter
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(o, Zone::Hand);
    if cx.source_on_battlefield(game) {
        add_counters_by(game, cx.source_id(), "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}

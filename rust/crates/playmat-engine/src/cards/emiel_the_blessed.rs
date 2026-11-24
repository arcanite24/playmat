//! Emiel the Blessed: "Whenever another creature you control enters, you may pay {G/W}. If you do, put a +1/+1
//! counter on it. If it's a Unicorn, put two +1/+1 counters on it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EmielTheBlessed.EmielTheBlessedEffect", counters)];

/// The creature that entered (the trigger's target pointer), if it's still that permanent: one +1/+1 counter, two
/// on a Unicorn
fn counters(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let n = if game.has_subtype(o, "Unicorn") { 2 } else { 1 };
    add_counters_by(game, o, "+1/+1", n, Some(cx.controller()));
    Some(())
}

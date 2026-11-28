//! Lion Sash: "{W}: Exile target card from a graveyard. If it was a permanent card, put a +1/+1 counter on Lion Sash."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LionSash.LionSashEffect", apply)];

/// The target is exiled; a permanent card puts a counter on the source if it's still the permanent
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = cx.obj(0)?;
    game.move_to(card, Zone::Exile);
    if game.obj(card).def.is_permanent_card() && cx.source_on_battlefield(game) {
        put_counters(game, cx.source_id(), "+1/+1", 1, cx.controller());
    }
    Some(())
}

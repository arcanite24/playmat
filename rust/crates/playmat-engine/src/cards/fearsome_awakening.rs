//! Fearsome Awakening: "Return target creature card from your graveyard to the battlefield. If it's a Dragon, put
//! two +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FearsomeAwakening.FearsomeAwakeningEffect", apply)];

/// `game.getPermanent(source.getFirstTarget())`: the card the return put onto the battlefield, a Dragon there
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Target::Obj(o) = *cx.item.targets.first()?.first()? else { return None };
    if game.obj(o).zone == Zone::Battlefield && game.has_subtype(o, "Dragon") {
        add_counters_by(game, o, "+1/+1", 2, Some(cx.controller()));
    }
    Some(())
}

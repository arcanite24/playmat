//! `ExileSourceEffect()`: "exile {this}" (Skyfisher Spider's "if you do, exile it from your graveyard").

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExileSourceEffect", apply)];

/// `getSourceCardIfItStillExists`: the source card, while it's the object the ability came from (its zone-change
/// count as the ability was put on the stack), into exile
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (o, zcc) = cx.source;
    if game.obj(o).zcc != zcc || game.obj(o).token || matches!(game.obj(o).zone, Zone::Exile | Zone::Library) {
        return None;
    }
    game.move_to(o, Zone::Exile);
    Some(())
}

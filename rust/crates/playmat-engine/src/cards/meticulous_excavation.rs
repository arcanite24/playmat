//! Meticulous Excavation: "{2}{W}: Return target permanent you control to its owner's hand. If it has unearth,
//! instead exile it, then return that card to its owner's hand. Activate only during your turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MeticulousExcavation.MeticulousExcavationEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let unearth = game.obj(o).def.abilities.iter().any(|a| {
        matches!(a, AbilityDef::GraveyardActivatedAbility { effects, .. }
            if effects.iter().any(|e| matches!(e, EffectDef::UnearthEffect)))
    });
    if unearth {
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile {
            game.move_to(o, Zone::Hand);
        }
    } else {
        game.move_to(o, Zone::Hand);
    }
    Some(())
}
